use crate::error::validate_node_id;
use crate::{BuildError, Graph, NodeId, Reorder};

/// Mutable builder for an immutable [`Graph`].
///
/// The builder accepts nodes and directed edges in insertion order. Calling
/// [`build`](Self::build) consumes the builder and compacts the data into the
/// graph's CSR layout.
#[derive(Clone, Debug)]
pub struct GraphBuilder<N, E> {
    nodes: Vec<N>,
    edges: Vec<(NodeId, NodeId, E)>,
    reorder: Reorder,
}

impl<N, E> GraphBuilder<N, E> {
    /// Creates an empty graph builder.
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            reorder: Reorder::None,
        }
    }

    /// Creates an empty graph builder with storage reserved up front.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
            reorder: Reorder::None,
        }
    }

    /// Sets the node reordering strategy to apply during [`build`](Self::build).
    pub fn reorder(&mut self, reorder: Reorder) -> &mut Self {
        self.reorder = reorder;
        self
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
        Graph::from_edges(self.nodes, self.edges)?.into_reordered(self.reorder)
    }
}

impl<N, E> Default for GraphBuilder<N, E> {
    fn default() -> Self {
        Self::new()
    }
}
