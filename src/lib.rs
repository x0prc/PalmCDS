//! Cache-conscious graph data structures.

use core::fmt;

/// Stable identifier for a node in a [`Graph`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates a node identifier from a zero-based index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns this identifier as a zero-based `usize` index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns this identifier as its compact `u32` representation.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// A directed, immutable graph stored in compressed sparse row form.
///
/// Nodes are stored contiguously, and each node owns one contiguous range of
/// outgoing edges. This keeps full scans and neighbor traversal cache-friendly
/// compared to pointer-heavy graph layouts.
#[derive(Clone, Debug)]
pub struct Graph<N, E> {
    nodes: Vec<Node<N>>,
    edges: Vec<Edge<E>>,
}

#[derive(Clone, Debug)]
struct Node<N> {
    data: N,
    first_edge: u32,
    edge_count: u32,
}

#[derive(Clone, Debug)]
struct Edge<E> {
    target: NodeId,
    data: E,
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
    inner: core::slice::Iter<'a, Edge<E>>,
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
    inner: core::slice::Iter<'a, Edge<E>>,
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

impl<E> ExactSizeIterator for Neighbors<'_, E> {}

/// Error returned when compact graph construction fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildError {
    /// More than `u32::MAX` nodes were provided.
    TooManyNodes { count: usize },
    /// More than `u32::MAX` edges were provided.
    TooManyEdges { count: usize },
    /// An edge references a node that does not exist in the graph.
    InvalidNodeId { id: NodeId, node_count: usize },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyNodes { count } => {
                write!(f, "graph has {count} nodes, exceeding u32::MAX")
            }
            Self::TooManyEdges { count } => {
                write!(f, "graph has {count} edges, exceeding u32::MAX")
            }
            Self::InvalidNodeId { id, node_count } => {
                write!(
                    f,
                    "node id {} is out of bounds for {node_count} nodes",
                    id.as_u32()
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// Mutable builder for an immutable [`Graph`].
///
/// The builder accepts nodes and directed edges in insertion order. Calling
/// [`build`](Self::build) consumes the builder and compacts the data into the
/// graph's CSR layout.
#[derive(Clone, Debug)]
pub struct GraphBuilder<N, E> {
    nodes: Vec<N>,
    edges: Vec<(NodeId, NodeId, E)>,
}

impl<N, E> GraphBuilder<N, E> {
    /// Creates an empty graph builder.
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Creates an empty graph builder with storage reserved up front.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
        }
    }

    /// Adds a node payload and returns its stable node ID.
    pub fn add_node(&mut self, data: N) -> Result<NodeId, BuildError> {
        if self.nodes.len() == u32::MAX as usize {
            return Err(BuildError::TooManyNodes {
                count: self.nodes.len() + 1,
            });
        }

        let id = NodeId::new(self.nodes.len() as u32);
        self.nodes.push(data);
        Ok(id)
    }

    /// Adds a directed edge from `source` to `target`.
    pub fn add_edge(&mut self, source: NodeId, target: NodeId, data: E) -> Result<(), BuildError> {
        validate_node_id(source, self.nodes.len())?;
        validate_node_id(target, self.nodes.len())?;

        if self.edges.len() == u32::MAX as usize {
            return Err(BuildError::TooManyEdges {
                count: self.edges.len() + 1,
            });
        }

        self.edges.push((source, target, data));
        Ok(())
    }

    /// Returns the number of nodes currently held by the builder.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges currently held by the builder.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Consumes the builder and returns a compact immutable graph.
    pub fn build(self) -> Result<Graph<N, E>, BuildError> {
        Graph::from_edges(self.nodes, self.edges)
    }
}

impl<N, E> Default for GraphBuilder<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

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
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Edges {
            inner: self.edges[start..end].iter(),
        })
    }

    /// Returns outgoing neighbor node IDs for `source`, or `None` if the ID is out of bounds.
    pub fn neighbors(&self, source: NodeId) -> Option<Neighbors<'_, E>> {
        let node = self.nodes.get(source.index())?;
        let start = node.first_edge as usize;
        let end = start + node.edge_count as usize;

        Some(Neighbors {
            inner: self.edges[start..end].iter(),
        })
    }

    /// Returns the out-degree for `source`, or `None` if the ID is out of bounds.
    pub fn out_degree(&self, source: NodeId) -> Option<usize> {
        self.nodes
            .get(source.index())
            .map(|node| node.edge_count as usize)
    }
}

fn validate_node_id(id: NodeId, node_count: usize) -> Result<(), BuildError> {
    if id.index() < node_count {
        Ok(())
    } else {
        Err(BuildError::InvalidNodeId { id, node_count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_edges_should_store_nodes_and_edges() {
        let graph = Graph::from_edges(
            vec!["a", "b", "c"],
            [
                (NodeId::new(0), NodeId::new(1), 10),
                (NodeId::new(0), NodeId::new(2), 20),
                (NodeId::new(2), NodeId::new(0), 30),
            ],
        )
        .unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 3);
        assert_eq!(graph.node_data(NodeId::new(1)), Some(&"b"));
    }

    #[test]
    fn edges_from_should_return_contiguous_outgoing_edges() {
        let graph = Graph::from_edges(
            vec![(), (), ()],
            [
                (NodeId::new(2), NodeId::new(0), "c-a"),
                (NodeId::new(0), NodeId::new(1), "a-b"),
                (NodeId::new(0), NodeId::new(2), "a-c"),
            ],
        )
        .unwrap();

        let edges: Vec<_> = graph
            .edges_from(NodeId::new(0))
            .unwrap()
            .map(|edge| (edge.target(), *edge.data()))
            .collect();

        assert_eq!(edges, [(NodeId::new(1), "a-b"), (NodeId::new(2), "a-c")]);
    }

    #[test]
    fn empty_outgoing_range_should_be_supported() {
        let graph = Graph::<_, ()>::from_edges(vec!["a", "b"], []).unwrap();

        assert_eq!(graph.out_degree(NodeId::new(1)), Some(0));
        assert_eq!(graph.edges_from(NodeId::new(1)).unwrap().len(), 0);
    }

    #[test]
    fn from_edges_should_reject_invalid_sources() {
        let err = Graph::from_edges(vec![()], [(NodeId::new(1), NodeId::new(0), ())]).unwrap_err();

        assert_eq!(
            err,
            BuildError::InvalidNodeId {
                id: NodeId::new(1),
                node_count: 1,
            }
        );
    }

    #[test]
    fn from_edges_should_reject_invalid_targets() {
        let err = Graph::from_edges(vec![()], [(NodeId::new(0), NodeId::new(1), ())]).unwrap_err();

        assert_eq!(
            err,
            BuildError::InvalidNodeId {
                id: NodeId::new(1),
                node_count: 1,
            }
        );
    }

    #[test]
    fn accessors_should_return_none_for_invalid_node_ids() {
        let graph = Graph::<_, ()>::from_edges(vec!["a"], []).unwrap();

        assert_eq!(graph.node_data(NodeId::new(2)), None);
        assert!(graph.edges_from(NodeId::new(2)).is_none());
        assert!(graph.neighbors(NodeId::new(2)).is_none());
        assert_eq!(graph.out_degree(NodeId::new(2)), None);
    }

    #[test]
    fn neighbors_should_return_outgoing_targets_only() {
        let graph = Graph::from_edges(
            vec![(), (), ()],
            [
                (NodeId::new(2), NodeId::new(0), "c-a"),
                (NodeId::new(0), NodeId::new(1), "a-b"),
                (NodeId::new(0), NodeId::new(2), "a-c"),
            ],
        )
        .unwrap();

        let neighbors: Vec<_> = graph.neighbors(NodeId::new(0)).unwrap().collect();

        assert_eq!(neighbors, [NodeId::new(1), NodeId::new(2)]);
    }

    #[test]
    fn neighbors_should_support_empty_outgoing_ranges() {
        let graph = Graph::<_, ()>::from_edges(vec!["a", "b"], []).unwrap();
        let neighbors = graph.neighbors(NodeId::new(1)).unwrap();

        assert_eq!(neighbors.len(), 0);
    }

    #[test]
    fn neighbors_should_report_exact_remaining_length() {
        let graph = Graph::from_edges(
            vec![(), (), ()],
            [
                (NodeId::new(0), NodeId::new(1), ()),
                (NodeId::new(0), NodeId::new(2), ()),
            ],
        )
        .unwrap();
        let mut neighbors = graph.neighbors(NodeId::new(0)).unwrap();

        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors.next(), Some(NodeId::new(1)));
        assert_eq!(neighbors.len(), 1);
    }

    #[test]
    fn builder_should_build_compact_graph() {
        let mut builder = GraphBuilder::new();
        let a = builder.add_node("a").unwrap();
        let b = builder.add_node("b").unwrap();
        let c = builder.add_node("c").unwrap();

        builder.add_edge(a, b, 10).unwrap();
        builder.add_edge(a, c, 20).unwrap();

        let graph = builder.build().unwrap();
        let edges: Vec<_> = graph
            .edges_from(a)
            .unwrap()
            .map(|edge| (edge.target(), *edge.data()))
            .collect();

        assert_eq!(edges, [(b, 10), (c, 20)]);
    }

    #[test]
    fn builder_should_track_pending_counts() {
        let mut builder = GraphBuilder::<_, ()>::with_capacity(2, 1);
        let a = builder.add_node("a").unwrap();
        let b = builder.add_node("b").unwrap();

        builder.add_edge(a, b, ()).unwrap();

        assert_eq!(builder.node_count(), 2);
        assert_eq!(builder.edge_count(), 1);
    }

    #[test]
    fn builder_should_reject_invalid_edges_before_build() {
        let mut builder = GraphBuilder::new();
        let a = builder.add_node("a").unwrap();

        let err = builder.add_edge(a, NodeId::new(1), ()).unwrap_err();

        assert_eq!(
            err,
            BuildError::InvalidNodeId {
                id: NodeId::new(1),
                node_count: 1,
            }
        );
    }

    #[test]
    fn graph_should_create_builder_for_matching_payload_types() {
        let mut builder = Graph::<&str, i32>::builder();
        let a = builder.add_node("a").unwrap();
        let b = builder.add_node("b").unwrap();

        builder.add_edge(a, b, 5).unwrap();

        assert_eq!(builder.build().unwrap().edge_count(), 1);
    }
}
