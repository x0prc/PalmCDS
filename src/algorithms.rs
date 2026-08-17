use crate::error::validate_node_id;
use crate::{Graph, NodeId};
use core::cmp::Ordering;
use core::fmt;
use std::collections::{BinaryHeap, VecDeque};

/// Error indicating a graph contains a cycle when a Directed Acyclic Graph (DAG) was expected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CycleError;

impl fmt::Display for CycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "graph contains at least one cycle")
    }
}

impl std::error::Error for CycleError {}

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: u64,
    node: NodeId,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.cost.cmp(&self.cost).then_with(|| self.node.cmp(&other.node))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<N, E> Graph<N, E> {
    /// Computes a global topological ordering of all nodes in a Directed Acyclic Graph (DAG).
    ///
    /// Returns `Err(CycleError)` if the graph contains any directed cycle.
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, CycleError> {
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

        while let Some(node) = queue.pop_front() {
            order.push(node);

            if let Some(neighbors) = self.neighbors(node) {
                for neighbor in neighbors {
                    let idx = neighbor.index();
                    in_degrees[idx] = in_degrees[idx].saturating_sub(1);
                    if in_degrees[idx] == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if order.len() == n {
            Ok(order)
        } else {
            Err(CycleError)
        }
    }

    /// Returns `true` if the graph contains at least one directed cycle.
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    /// Computes single-source shortest path distances using Dijkstra's algorithm.
    ///
    /// `edge_weight` is a closure mapping an edge payload `&E` to a non-negative `u64` weight.
    /// Returns `None` if `start` is an invalid `NodeId`.
    pub fn dijkstra<F>(&self, start: NodeId, mut edge_weight: F) -> Option<Vec<Option<u64>>>
    where
        F: FnMut(&E) -> u64,
    {
        validate_node_id(start, self.node_count()).ok()?;

        let n = self.node_count();
        let mut dists = vec![None; n];
        let mut heap = BinaryHeap::new();

        dists[start.index()] = Some(0);
        heap.push(State {
            cost: 0,
            node: start,
        });

        while let Some(State { cost, node }) = heap.pop() {
            if let Some(current_dist) = dists[node.index()] {
                if cost > current_dist {
                    continue;
                }
            }

            if let Some(edges) = self.edges_from(node) {
                for edge in edges {
                    let target = edge.target();
                    let weight = edge_weight(edge.data());
                    let next_dist = cost.saturating_add(weight);

                    let target_idx = target.index();
                    let is_shorter = match dists[target_idx] {
                        Some(d) => next_dist < d,
                        None => true,
                    };

                    if is_shorter {
                        dists[target_idx] = Some(next_dist);
                        heap.push(State {
                            cost: next_dist,
                            node: target,
                        });
                    }
                }
            }
        }

        Some(dists)
    }

    /// Computes all Strongly Connected Components (SCCs) using Tarjan's algorithm.
    ///
    /// Each component is a vector of `NodeId`s that can reach every other node in the same component.
    pub fn strongly_connected_components(&self) -> Vec<Vec<NodeId>> {
        let n = self.node_count();
        let mut tarjan = TarjanState {
            graph: self,
            index: 0,
            indices: vec![None; n],
            lowlink: vec![0; n],
            on_stack: vec![false; n],
            stack: Vec::new(),
            sccs: Vec::new(),
        };

        for i in 0..n {
            if tarjan.indices[i].is_none() {
                tarjan.strongconnect(NodeId::new(i as u32));
            }
        }

        tarjan.sccs
    }
}

struct TarjanState<'a, N, E> {
    graph: &'a Graph<N, E>,
    index: usize,
    indices: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<NodeId>,
    sccs: Vec<Vec<NodeId>>,
}

impl<N, E> TarjanState<'_, N, E> {
    fn strongconnect(&mut self, u: NodeId) {
        let u_idx = u.index();
        self.indices[u_idx] = Some(self.index);
        self.lowlink[u_idx] = self.index;
        self.index += 1;

        self.stack.push(u);
        self.on_stack[u_idx] = true;

        if let Some(neighbors) = self.graph.neighbors(u) {
            for v in neighbors {
                let v_idx = v.index();
                if self.indices[v_idx].is_none() {
                    self.strongconnect(v);
                    self.lowlink[u_idx] = self.lowlink[u_idx].min(self.lowlink[v_idx]);
                } else if self.on_stack[v_idx] {
                    let v_index = self.indices[v_idx].unwrap();
                    self.lowlink[u_idx] = self.lowlink[u_idx].min(v_index);
                }
            }
        }

        if self.indices[u_idx] == Some(self.lowlink[u_idx]) {
            let mut scc = Vec::new();
            while let Some(w) = self.stack.pop() {
                let w_idx = w.index();
                self.on_stack[w_idx] = false;
                scc.push(w);
                if w == u {
                    break;
                }
            }
            self.sccs.push(scc);
        }
    }
}
