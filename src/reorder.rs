use crate::error::validate_node_id;
use crate::graph::Node;
use crate::{BuildError, Graph, NodeId};
use std::collections::VecDeque;

/// Node ordering strategy used when compacting or rebuilding a graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reorder {
    /// Keep node IDs in their current order.
    None,
    /// Relabel nodes in breadth-first visitation order from `root`.
    Bfs { root: NodeId },
}

impl<N, E> Graph<N, E> {
    /// Returns a reordered clone of this graph.
    pub fn reordered(&self, reorder: Reorder) -> Result<Self, BuildError>
    where
        N: Clone,
        E: Clone,
    {
        self.clone().into_reordered(reorder)
    }

    /// Consumes this graph and returns a graph with node IDs relabeled by `reorder`.
    pub fn into_reordered(self, reorder: Reorder) -> Result<Self, BuildError> {
        match reorder {
            Reorder::None => Ok(self),
            Reorder::Bfs { root } => self.into_bfs_reordered(root),
        }
    }

    fn bfs_reorder_order(&self, root: NodeId) -> Result<Vec<NodeId>, BuildError> {
        validate_node_id(root, self.node_count())?;

        let mut visited = vec![false; self.node_count()];
        let mut queue = VecDeque::new();
        let mut order = Vec::with_capacity(self.node_count());

        visited[root.index()] = true;
        queue.push_back(root);

        while let Some(node) = queue.pop_front() {
            order.push(node);

            if let Some(neighbors) = self.neighbors(node) {
                for neighbor in neighbors {
                    let index = neighbor.index();
                    if !visited[index] {
                        visited[index] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // A root may not reach the whole graph. Keep the output dense and
        // deterministic by preserving original order for disconnected nodes.
        for (index, is_visited) in visited.iter().enumerate() {
            if !is_visited {
                order.push(NodeId::new(index as u32));
            }
        }

        Ok(order)
    }

    fn into_bfs_reordered(self, root: NodeId) -> Result<Self, BuildError> {
        let order = self.bfs_reorder_order(root)?;
        self.into_reordered_by_order(order)
    }

    fn into_reordered_by_order(self, order: Vec<NodeId>) -> Result<Self, BuildError> {
        let Graph { nodes, edges } = self;
        let mut old_to_new = vec![NodeId::new(0); nodes.len()];

        for (new_index, old_id) in order.iter().copied().enumerate() {
            old_to_new[old_id.index()] = NodeId::new(new_index as u32);
        }

        let mut edge_sources = Vec::with_capacity(edges.len());
        for (source_index, node) in nodes.iter().enumerate() {
            for _ in 0..node.edge_count {
                edge_sources.push(NodeId::new(source_index as u32));
            }
        }

        let reordered_edges = edge_sources.into_iter().zip(edges).map(|(source, edge)| {
            (
                old_to_new[source.index()],
                old_to_new[edge.target.index()],
                edge.data,
            )
        });

        let mut old_nodes: Vec<Option<Node<N>>> = nodes.into_iter().map(Some).collect();
        let mut reordered_nodes = Vec::with_capacity(old_nodes.len());

        for old_id in order {
            let node = old_nodes[old_id.index()]
                .take()
                .expect("reorder order should contain each node exactly once");
            reordered_nodes.push(node.data);
        }

        Graph::from_edges(reordered_nodes, reordered_edges)
    }
}
