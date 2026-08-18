use crate::error::validate_node_id;
use crate::graph::Node;
use crate::{BuildError, Graph, NodeId};
use std::collections::VecDeque;

/// Node ordering strategy used when compacting or rebuilding a graph.
///
/// Reordering relabels `NodeId`s so that nodes frequently accessed together
/// reside near each other in memory, maximizing CPU cache line locality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reorder {
    /// Keep node IDs in their current order.
    None,
    /// Relabel nodes in breadth-first visitation order starting from `root`.
    Bfs { root: NodeId },
    /// Relabel nodes in depth-first visitation order starting from `root`.
    Dfs { root: NodeId },
    /// Relabel nodes in reverse post-order starting from `root`.
    ///
    /// Ideal for DAGs and control-flow graphs, placing dependencies before consumers.
    ReversePostOrder { root: NodeId },
    /// Relabel nodes in global topological order.
    ///
    /// If cycles exist, unvisited cyclic nodes are appended in original ID order.
    Topological,
    /// Reverse Cuthill-McKee (RCM) ordering.
    ///
    /// Reduces graph adjacency bandwidth, keeping connected neighbors tightly packed
    /// in cache. If `root` is `None`, the node with the lowest out-degree is chosen.
    ReverseCuthillMcKee { root: Option<NodeId> },
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
            Reorder::Dfs { root } => self.into_dfs_reordered(root),
            Reorder::ReversePostOrder { root } => self.into_rpo_reordered(root),
            Reorder::Topological => self.into_topological_reordered(),
            Reorder::ReverseCuthillMcKee { root } => self.into_rcm_reordered(root),
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

        // Preserve original order for any disconnected nodes unreachable from root.
        for (index, is_visited) in visited.iter().enumerate() {
            if !is_visited {
                order.push(NodeId::new(index as u32));
            }
        }

        Ok(order)
    }

    fn dfs_reorder_order(&self, root: NodeId) -> Result<Vec<NodeId>, BuildError> {
        validate_node_id(root, self.node_count())?;

        let mut visited = vec![false; self.node_count()];
        let mut stack = vec![root];
        let mut order = Vec::with_capacity(self.node_count());

        visited[root.index()] = true;

        while let Some(node) = stack.pop() {
            order.push(node);

            if let Some(neighbors) = self.neighbors(node) {
                for neighbor in neighbors.rev() {
                    let index = neighbor.index();
                    if !visited[index] {
                        visited[index] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }

        for (index, is_visited) in visited.iter().enumerate() {
            if !is_visited {
                order.push(NodeId::new(index as u32));
            }
        }

        Ok(order)
    }

    fn rpo_reorder_order(&self, root: NodeId) -> Result<Vec<NodeId>, BuildError> {
        validate_node_id(root, self.node_count())?;

        let mut visited = vec![false; self.node_count()];
        let mut post_order = Vec::with_capacity(self.node_count());

        // Explicit post-order DFS stack to avoid recursion limit / stack overflow.
        let mut stack: Vec<(NodeId, usize)> = vec![(root, 0)];
        visited[root.index()] = true;

        while let Some((node, neighbor_idx)) = stack.last_mut() {
            let curr_node = *node;
            let mut pushed_child = false;

            if let Some(neighbors) = self.neighbors(curr_node) {
                let neighbor_vec: Vec<NodeId> = neighbors.collect();
                while *neighbor_idx < neighbor_vec.len() {
                    let neighbor = neighbor_vec[*neighbor_idx];
                    *neighbor_idx += 1;

                    let idx = neighbor.index();
                    if !visited[idx] {
                        visited[idx] = true;
                        stack.push((neighbor, 0));
                        pushed_child = true;
                        break;
                    }
                }
            }

            if !pushed_child {
                post_order.push(curr_node);
                stack.pop();
            }
        }

        // Reverse post-order
        post_order.reverse();

        for (index, is_visited) in visited.iter().enumerate() {
            if !is_visited {
                post_order.push(NodeId::new(index as u32));
            }
        }

        Ok(post_order)
    }

    fn topological_reorder_order(&self) -> Vec<NodeId> {
        let n = self.node_count();
        let mut in_degrees = vec![0u32; n];

        for i in 0..n {
            if let Some(neighbors) = self.neighbors(NodeId::new(i as u32)) {
                for target in neighbors {
                    in_degrees[target.index()] += 1;
                }
            }
        }

        let mut queue = VecDeque::new();
        for (i, &deg) in in_degrees.iter().enumerate() {
            if deg == 0 {
                queue.push_back(NodeId::new(i as u32));
            }
        }

        let mut order = Vec::with_capacity(n);
        let mut visited = vec![false; n];

        while let Some(node) = queue.pop_front() {
            visited[node.index()] = true;
            order.push(node);

            if let Some(neighbors) = self.neighbors(node) {
                for neighbor in neighbors {
                    let idx = neighbor.index();
                    in_degrees[idx] = in_degrees[idx].saturating_sub(1);
                    if in_degrees[idx] == 0 && !visited[idx] {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Append any remaining unvisited (e.g. part of cycles)
        for (index, is_visited) in visited.iter().enumerate() {
            if !is_visited {
                order.push(NodeId::new(index as u32));
            }
        }

        order
    }

    fn rcm_reorder_order(&self, start_root: Option<NodeId>) -> Result<Vec<NodeId>, BuildError> {
        if let Some(r) = start_root {
            validate_node_id(r, self.node_count())?;
        }

        let n = self.node_count();
        if n == 0 {
            return Ok(Vec::new());
        }

        let mut visited = vec![false; n];
        let mut order = Vec::with_capacity(n);

        // Calculate degrees for sorting neighbors
        let degrees: Vec<usize> = (0..n)
            .map(|i| self.out_degree(NodeId::new(i as u32)).unwrap_or(0))
            .collect();

        let find_unvisited_lowest_degree = |visited: &[bool]| -> Option<NodeId> {
            let mut best_node = None;
            let mut min_deg = usize::MAX;

            for i in 0..n {
                if !visited[i] && degrees[i] < min_deg {
                    min_deg = degrees[i];
                    best_node = Some(NodeId::new(i as u32));
                }
            }

            best_node
        };

        let mut next_start = start_root.or_else(|| find_unvisited_lowest_degree(&visited));

        while let Some(start) = next_start {
            if !visited[start.index()] {
                visited[start.index()] = true;
                let mut cm_queue = VecDeque::new();
                cm_queue.push_back(start);

                while let Some(node) = cm_queue.pop_front() {
                    order.push(node);

                    if let Some(neighbors) = self.neighbors(node) {
                        let mut unvisited_neighbors: Vec<NodeId> =
                            neighbors.filter(|nbr| !visited[nbr.index()]).collect();

                        // Sort unvisited neighbors by degree ascending
                        unvisited_neighbors.sort_by_key(|nbr| degrees[nbr.index()]);

                        for nbr in unvisited_neighbors {
                            if !visited[nbr.index()] {
                                visited[nbr.index()] = true;
                                cm_queue.push_back(nbr);
                            }
                        }
                    }
                }
            }

            next_start = find_unvisited_lowest_degree(&visited);
        }

        // Reverse Cuthill-McKee
        order.reverse();
        Ok(order)
    }

    fn into_bfs_reordered(self, root: NodeId) -> Result<Self, BuildError> {
        let order = self.bfs_reorder_order(root)?;
        self.into_reordered_by_order(order)
    }

    fn into_dfs_reordered(self, root: NodeId) -> Result<Self, BuildError> {
        let order = self.dfs_reorder_order(root)?;
        self.into_reordered_by_order(order)
    }

    fn into_rpo_reordered(self, root: NodeId) -> Result<Self, BuildError> {
        let order = self.rpo_reorder_order(root)?;
        self.into_reordered_by_order(order)
    }

    fn into_topological_reordered(self) -> Result<Self, BuildError> {
        let order = self.topological_reorder_order();
        self.into_reordered_by_order(order)
    }

    fn into_rcm_reordered(self, root: Option<NodeId>) -> Result<Self, BuildError> {
        let order = self.rcm_reorder_order(root)?;
        self.into_reordered_by_order(order)
    }

    fn into_reordered_by_order(self, order: Vec<NodeId>) -> Result<Self, BuildError> {
        let Graph {
            nodes,
            edge_targets,
            edge_data,
        } = self;

        let mut old_to_new = vec![NodeId::new(0); nodes.len()];
        for (new_index, old_id) in order.iter().copied().enumerate() {
            old_to_new[old_id.index()] = NodeId::new(new_index as u32);
        }

        let mut edge_sources = Vec::with_capacity(edge_targets.len());
        for (source_index, node) in nodes.iter().enumerate() {
            for _ in 0..node.edge_count {
                edge_sources.push(NodeId::new(source_index as u32));
            }
        }

        let reordered_edges = edge_sources
            .into_iter()
            .zip(edge_targets)
            .zip(edge_data)
            .map(|((source, target), data)| {
                (old_to_new[source.index()], old_to_new[target.index()], data)
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
