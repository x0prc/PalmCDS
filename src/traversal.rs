use crate::{Graph, NodeId};
use std::collections::VecDeque;

/// Breadth-first traversal over reachable node IDs.
#[derive(Clone, Debug)]
pub struct Bfs<'a, N, E> {
    graph: &'a Graph<N, E>,
    visited: Vec<bool>,
    queue: VecDeque<NodeId>,
}

impl<'a, N, E> Bfs<'a, N, E> {
    pub(crate) fn new(graph: &'a Graph<N, E>, start: NodeId) -> Option<Self> {
        graph.node_data(start)?;

        let mut visited = vec![false; graph.node_count()];
        visited[start.index()] = true;

        let mut queue = VecDeque::new();
        queue.push_back(start);

        Some(Self {
            graph,
            visited,
            queue,
        })
    }
}

impl<N, E> Iterator for Bfs<'_, N, E> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_front()?;

        // Mark neighbors when they are enqueued, not when they are popped. This
        // avoids pushing the same node multiple times when several incoming
        // edges reach it from the current frontier.
        if let Some(neighbors) = self.graph.neighbors(node) {
            for neighbor in neighbors {
                let index = neighbor.index();
                if !self.visited[index] {
                    self.visited[index] = true;
                    self.queue.push_back(neighbor);
                }
            }
        }

        Some(node)
    }
}

/// Depth-first traversal over reachable node IDs.
#[derive(Clone, Debug)]
pub struct Dfs<'a, N, E> {
    graph: &'a Graph<N, E>,
    visited: Vec<bool>,
    stack: Vec<NodeId>,
}

impl<'a, N, E> Dfs<'a, N, E> {
    pub(crate) fn new(graph: &'a Graph<N, E>, start: NodeId) -> Option<Self> {
        graph.node_data(start)?;

        let mut visited = vec![false; graph.node_count()];
        visited[start.index()] = true;

        Some(Self {
            graph,
            visited,
            stack: vec![start],
        })
    }
}

impl<N, E> Iterator for Dfs<'_, N, E> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        // The stack is LIFO. Iterating neighbors from the back means the first
        // neighbor in adjacency order is popped first on the next step.
        if let Some(neighbors) = self.graph.neighbors(node) {
            for neighbor in neighbors.rev() {
                let index = neighbor.index();
                if !self.visited[index] {
                    self.visited[index] = true;
                    self.stack.push(neighbor);
                }
            }
        }

        Some(node)
    }
}
