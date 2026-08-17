use crate::error::validate_node_id;
use crate::{Bfs, BuildError, Dfs, GraphBuilder, NodeId};
use core::mem::size_of;

/// A directed, immutable graph stored in compressed sparse row (CSR) form using
/// Structure-of-Arrays (SoA) edge storage for maximal L1/L2 CPU cache utilization.
///
/// Nodes are stored contiguously in `nodes`. Edge targets are stored in a contiguous,
/// densely-packed `edge_targets` array (`NodeId`s only), allowing neighbor scans,
/// BFS, DFS, and topological reordering to run at full memory bandwidth without
/// loading edge payloads `E` into cache lines.
#[derive(Clone, Debug)]
pub struct Graph<N, E> {
    // CSR header storage. Each node records where its outgoing edge range
    // starts inside `edge_targets`/`edge_data` and how many entries belong to that range.
    pub(crate) nodes: Vec<Node<N>>,
    // Dense contiguous array of edge target IDs (4 bytes each).
    pub(crate) edge_targets: Vec<NodeId>,
    // Dense contiguous array of edge payloads.
    pub(crate) edge_data: Vec<E>,
}

#[derive(Clone, Debug)]
pub(crate) struct Node<N> {
    pub(crate) data: N,
    // Offset into edge_targets/edge_data for this node's first outgoing edge.
    pub(crate) first_edge: u32,
    // Number of edges in this node's contiguous outgoing range.
    pub(crate) edge_count: u32,
}

/// Borrowed view of one outgoing edge.
#[derive(Debug)]
pub struct EdgeRef<'a, E> {
    target: NodeId,
    data: &'a E,
}

impl<E> Clone for EdgeRef<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for EdgeRef<'_, E> {}

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
    pub(crate) targets: core::slice::Iter<'a, NodeId>,
    pub(crate) data: core::slice::Iter<'a, E>,
}

impl<'a, E> Iterator for Edges<'a, E> {
    type Item = EdgeRef<'a, E>;

    fn next(&mut self) -> Option<Self::Item> {
        let target = *self.targets.next()?;
        let data = self.data.next()?;
        Some(EdgeRef { target, data })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.targets.size_hint()
    }
}

impl<E> ExactSizeIterator for Edges<'_, E> {}

/// Iterator over outgoing neighbor node IDs.
#[derive(Clone, Debug)]
pub struct Neighbors<'a> {
    pub(crate) inner: core::slice::Iter<'a, NodeId>,
}

impl Iterator for Neighbors<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for Neighbors<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().copied()
    }
}

impl ExactSizeIterator for Neighbors<'_> {}

impl<N, E> Graph<N, E> {
    /// Creates an empty mutable builder for this graph type.
    pub const fn builder() -> GraphBuilder<N, E> {
        GraphBuilder::new()
    }

    /// Builds an immutable directed graph from node payloads and directed edges.
    ///
    /// The input edges are `(source, target, payload)` triples. The resulting
    /// graph groups outgoing edges by source node into contiguous ranges in CSR format.
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

        // Sort edges by source so every node points to a single contiguous slice.
        edges.sort_by_key(|(source, _, _)| *source);

        let mut graph_nodes: Vec<_> = nodes
            .into_iter()
            .map(|data| Node {
                data,
                first_edge: 0,
                edge_count: 0,
            })
            .collect();

        let edge_count = edges.len();
        let mut edge_targets = Vec::with_capacity(edge_count);
        let mut edge_data = Vec::with_capacity(edge_count);

        for (source, target, data) in edges {
            let source_index = source.index();
            let node = &mut graph_nodes[source_index];

            if node.edge_count == 0 {
                node.first_edge = edge_targets.len() as u32;
            }

            node.edge_count += 1;
            edge_targets.push(target);
            edge_data.push(data);
        }

        Ok(Self {
            nodes: graph_nodes,
            edge_targets,
            edge_data,
        })
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edge_targets.len()
    }

    /// Returns true when the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the size of one internal node header plus its payload.
    pub const fn node_entry_size() -> usize {
        size_of::<Node<N>>()
    }

    /// Returns the size of one internal edge target entry (4 bytes).
    pub const fn edge_target_size() -> usize {
        size_of::<NodeId>()
    }

    /// Returns the size of one edge payload `E`.
    pub const fn edge_payload_size() -> usize {
        size_of::<E>()
    }

    /// Returns bytes allocated for the node arena.
    pub fn node_storage_bytes(&self) -> usize {
        self.nodes.capacity() * Self::node_entry_size()
    }

    /// Returns bytes allocated for the edge target and payload arenas.
    pub fn edge_storage_bytes(&self) -> usize {
        self.edge_targets.capacity() * Self::edge_target_size()
            + self.edge_data.capacity() * Self::edge_payload_size()
    }

    /// Returns total bytes allocated across all graph arenas.
    pub fn total_storage_bytes(&self) -> usize {
        self.node_storage_bytes() + self.edge_storage_bytes()
    }

    /// Returns the node payload for `id`, or `None` if the ID is out of bounds.
    pub fn node_data(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(id.index()).map(|node| &node.data)
    }

    /// Returns outgoing edges for `source`, or `None` if the ID is out of bounds.
    pub fn edges_from(&self, source: NodeId) -> Option<Edges<'_, E>> {
        let node = self.nodes.get(source.index())?;
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Edges {
            targets: self.edge_targets[start..end].iter(),
            data: self.edge_data[start..end].iter(),
        })
    }

    /// Returns outgoing neighbor node IDs for `source`, or `None` if the ID is out of bounds.
    pub fn neighbors(&self, source: NodeId) -> Option<Neighbors<'_>> {
        let node = self.nodes.get(source.index())?;
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Neighbors {
            inner: self.edge_targets[start..end].iter(),
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
