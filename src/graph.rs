use crate::error::validate_node_id;
use crate::{Bfs, BuildError, Dfs, GraphBuilder, NodeId};

/// A directed, immutable graph stored in compressed sparse row form.
///
/// Nodes are stored contiguously, and each node owns one contiguous range of
/// outgoing edges. This keeps full scans and neighbor traversal cache-friendly
/// compared to pointer-heavy graph layouts.
#[derive(Clone, Debug)]
pub struct Graph<N, E> {
    // CSR header storage. Each node records where its outgoing edge range
    // starts inside `edges` and how many entries belong to that range.
    pub(crate) nodes: Vec<Node<N>>,
    // Flat edge arena. Edges for the same source node are stored next to each
    // other, which is the main locality benefit of this representation.
    pub(crate) edges: Vec<Edge<E>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Node<N> {
    pub(crate) data: N,
    pub(crate) first_edge: u32,
    pub(crate) edge_count: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct Edge<E> {
    pub(crate) target: NodeId,
    pub(crate) data: E,
}

/// Borrowed view of one outgoing edge.
#[derive(Clone, Copy, Debug)]
pub struct EdgeRef<'a, E> {
    target: NodeId,
    data: &'a E,
}

impl<'a, E> EdgeRef<'a, E> {
    /// Returns the edge target.
    pub const fn target(self) -> NodeId {
        self.target
    }

    /// Returns the edge payload.
    pub const fn data(self) -> &'a E {
        self.data
    }
}

/// Iterator over the outgoing edges for a node.
#[derive(Clone, Debug)]
pub struct Edges<'a, E> {
    // This borrows a precomputed contiguous slice from the graph's edge arena;
    // advancing the iterator does not allocate or chase per-node heap objects.
    pub(crate) inner: core::slice::Iter<'a, Edge<E>>,
}

impl<'a, E> Iterator for Edges<'a, E> {
    type Item = EdgeRef<'a, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|edge| EdgeRef {
            target: edge.target,
            data: &edge.data,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<E> ExactSizeIterator for Edges<'_, E> {}

/// Iterator over outgoing neighbor node IDs.
#[derive(Clone, Debug)]
pub struct Neighbors<'a, E> {
    // Same backing range as Edges, but yields only targets. Traversal algorithms
    // can avoid touching edge payloads when they only need topology.
    pub(crate) inner: core::slice::Iter<'a, Edge<E>>,
}

impl<E> Iterator for Neighbors<'_, E> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|edge| edge.target)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<E> DoubleEndedIterator for Neighbors<'_, E> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|edge| edge.target)
    }
}

impl<E> ExactSizeIterator for Neighbors<'_, E> {}

impl<N, E> Graph<N, E> {
    /// Creates an empty mutable builder for this graph type.
    pub const fn builder() -> GraphBuilder<N, E> {
        GraphBuilder::new()
    }

    /// Builds an immutable directed graph from node payloads and directed edges.
    ///
    /// The input edges are `(source, target, payload)` triples. The resulting
    /// graph groups outgoing edges by source node into contiguous ranges.
    pub fn from_edges(
        nodes: Vec<N>,
        edges: impl IntoIterator<Item = (NodeId, NodeId, E)>,
    ) -> Result<Self, BuildError> {
        if nodes.len() > u32::MAX as usize {
            return Err(BuildError::TooManyNodes { count: nodes.len() });
        }

        let node_count = nodes.len();
        let mut edges: Vec<_> = edges.into_iter().collect();

        if edges.len() > u32::MAX as usize {
            return Err(BuildError::TooManyEdges { count: edges.len() });
        }

        for (source, target, _) in &edges {
            validate_node_id(*source, node_count)?;
            validate_node_id(*target, node_count)?;
        }

        // Group by source so every node can point at a single contiguous range
        // in `graph_edges`. This is the CSR compaction step.
        edges.sort_by_key(|(source, _, _)| *source);

        let mut graph_nodes: Vec<_> = nodes
            .into_iter()
            .map(|data| Node {
                data,
                first_edge: 0,
                edge_count: 0,
            })
            .collect();
        let mut graph_edges = Vec::with_capacity(edges.len());

        for (source, target, data) in edges {
            let source_index = source.index();
            let node = &mut graph_nodes[source_index];

            // The first edge we see for a source records the start of that
            // source's range. Later edges only extend the range length.
            if node.edge_count == 0 {
                node.first_edge = graph_edges.len() as u32;
            }

            node.edge_count += 1;
            graph_edges.push(Edge { target, data });
        }

        Ok(Self {
            nodes: graph_nodes,
            edges: graph_edges,
        })
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true when the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the node payload for `id`, or `None` if the ID is out of bounds.
    pub fn node_data(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(id.index()).map(|node| &node.data)
    }

    /// Returns outgoing edges for `source`, or `None` if the ID is out of bounds.
    pub fn edges_from(&self, source: NodeId) -> Option<Edges<'_, E>> {
        let node = self.nodes.get(source.index())?;
        // `first_edge..first_edge + edge_count` is valid because from_edges only
        // creates ranges while pushing into the same edge arena.
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Edges {
            inner: self.edges[start..end].iter(),
        })
    }

    /// Returns outgoing neighbor node IDs for `source`, or `None` if the ID is out of bounds.
    pub fn neighbors(&self, source: NodeId) -> Option<Neighbors<'_, E>> {
        let node = self.nodes.get(source.index())?;
        // Neighbor iteration deliberately reuses the exact CSR slice used by
        // edges_from, but maps each edge down to its target NodeId.
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Neighbors {
            inner: self.edges[start..end].iter(),
        })
    }

    /// Returns a breadth-first traversal starting at `start`.
    pub fn bfs(&self, start: NodeId) -> Option<Bfs<'_, N, E>> {
        Bfs::new(self, start)
    }

    /// Returns a depth-first traversal starting at `start`.
    pub fn dfs(&self, start: NodeId) -> Option<Dfs<'_, N, E>> {
        Dfs::new(self, start)
    }

    /// Returns the out-degree for `source`, or `None` if the ID is out of bounds.
    pub fn out_degree(&self, source: NodeId) -> Option<usize> {
        self.nodes
            .get(source.index())
            .map(|node| node.edge_count as usize)
    }
}
