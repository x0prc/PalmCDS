use criterion::{Criterion, black_box, criterion_group, criterion_main};
use palmcds::{Graph, GraphBuilder, NodeId, Reorder};
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::VecDeque;
use std::mem::size_of;

const NODE_COUNT: usize = 10_000;
const GRID_SIDE: usize = 100;
const FANOUT: usize = 4;

const SHAPES: [GraphShape; 4] = [
    GraphShape::Chain,
    GraphShape::FixedFanoutRing,
    GraphShape::Grid,
    GraphShape::PermutedFanout,
];

#[derive(Clone, Copy, Debug)]
enum GraphShape {
    Chain,
    FixedFanoutRing,
    Grid,
    PermutedFanout,
}

impl GraphShape {
    fn name(self) -> &'static str {
        match self {
            Self::Chain => "chain",
            Self::FixedFanoutRing => "fixed_fanout_ring",
            Self::Grid => "grid",
            Self::PermutedFanout => "permuted_fanout",
        }
    }

    fn edge_capacity(self) -> usize {
        match self {
            Self::Chain => NODE_COUNT.saturating_sub(1),
            Self::FixedFanoutRing | Self::PermutedFanout => NODE_COUNT * FANOUT,
            Self::Grid => 2 * GRID_SIDE * (GRID_SIDE - 1),
        }
    }

    fn edges(self) -> Vec<(usize, usize)> {
        let mut edges = Vec::with_capacity(self.edge_capacity());

        match self {
            Self::Chain => {
                for source in 0..NODE_COUNT - 1 {
                    edges.push((source, source + 1));
                }
            }
            Self::FixedFanoutRing => {
                for source in 0..NODE_COUNT {
                    for offset in 1..=FANOUT {
                        edges.push((source, (source + offset) % NODE_COUNT));
                    }
                }
            }
            Self::Grid => {
                debug_assert_eq!(NODE_COUNT, GRID_SIDE * GRID_SIDE);

                for row in 0..GRID_SIDE {
                    for col in 0..GRID_SIDE {
                        let source = row * GRID_SIDE + col;

                        if col + 1 < GRID_SIDE {
                            edges.push((source, source + 1));
                        }

                        if row + 1 < GRID_SIDE {
                            edges.push((source, source + GRID_SIDE));
                        }
                    }
                }
            }
            Self::PermutedFanout => {
                for source in 0..NODE_COUNT {
                    // Always include the next node so BFS from 0 reaches the
                    // whole graph; the remaining edges add deterministic noise.
                    edges.push((source, (source + 1) % NODE_COUNT));

                    for offset in 1..FANOUT {
                        let target = source
                            .wrapping_mul(1_103_515_245)
                            .wrapping_add(offset * 2_654_435_761)
                            % NODE_COUNT;
                        edges.push((source, target));
                    }
                }
            }
        }

        edges
    }
}

fn build_graph(shape: GraphShape) -> Graph<(), ()> {
    let edges = shape.edges();
    let mut builder = GraphBuilder::with_capacity(NODE_COUNT, edges.len());
    let nodes: Vec<_> = (0..NODE_COUNT)
        .map(|_| builder.add_node(()).unwrap())
        .collect();

    for (source, target) in edges {
        builder.add_edge(nodes[source], nodes[target], ()).unwrap();
    }

    builder.build().unwrap()
}

// Use the same topology as build_graph, but ask PalmCDS to relabel nodes in BFS
// order during compaction. This isolates the cost and effect of reordering.
fn build_reordered_graph(shape: GraphShape) -> Graph<(), ()> {
    let edges = shape.edges();
    let mut builder = GraphBuilder::with_capacity(NODE_COUNT, edges.len());
    let nodes: Vec<_> = (0..NODE_COUNT)
        .map(|_| builder.add_node(()).unwrap())
        .collect();

    for (source, target) in edges {
        builder.add_edge(nodes[source], nodes[target], ()).unwrap();
    }

    builder.reorder(Reorder::Bfs { root: nodes[0] });
    builder.build().unwrap()
}

// Simple adjacency-list baseline. This is not meant to be a full competing
// graph library; it is the common Vec<Vec<NodeId>> shape many users start with.
fn build_adjacency_list(shape: GraphShape) -> Vec<Vec<NodeId>> {
    let edges = shape.edges();
    let mut degrees = vec![0; NODE_COUNT];

    for (source, _) in &edges {
        degrees[*source] += 1;
    }

    let mut adjacency = degrees
        .into_iter()
        .map(Vec::with_capacity)
        .collect::<Vec<_>>();

    for (source, target) in edges {
        adjacency[source].push(NodeId::new(target as u32));
    }

    adjacency
}

// Petgraph comparison using its common adjacency-list graph type. This gives us
// a reference point against an established Rust graph library on the same shape.
fn build_petgraph(shape: GraphShape) -> DiGraph<(), ()> {
    let edges = shape.edges();
    let mut graph = DiGraph::with_capacity(NODE_COUNT, edges.len());
    let nodes: Vec<_> = (0..NODE_COUNT).map(|_| graph.add_node(())).collect();

    for (source, target) in edges {
        graph.add_edge(nodes[source], nodes[target], ());
    }

    graph
}

// Estimate the memory owned by the Vec<Vec<NodeId>> baseline. This mirrors
// Graph::total_storage_bytes by using vector capacity and excluding allocator
// metadata. The outer Vec capacity is part of the baseline's storage, so this
// intentionally takes &Vec rather than a slice.
fn adjacency_storage_bytes(adjacency: &Vec<Vec<NodeId>>) -> usize {
    let outer_storage = adjacency.capacity() * size_of::<Vec<NodeId>>();
    let inner_storage = adjacency
        .iter()
        .map(|targets| targets.capacity() * size_of::<NodeId>())
        .sum::<usize>();

    outer_storage + inner_storage
}

// Scan every outgoing neighbor without touching edge payloads. The checksum
// prevents the optimizer from deleting the traversal work.
fn scan_graph_neighbors(graph: &Graph<(), ()>) -> u64 {
    let mut sum = 0;

    for source in 0..graph.node_count() {
        for neighbor in graph.neighbors(NodeId::new(source as u32)).unwrap() {
            sum += u64::from(neighbor.as_u32());
        }
    }

    sum
}

// Same scan as scan_graph_neighbors, but over the adjacency-list baseline.
fn scan_adjacency_neighbors(adjacency: &[Vec<NodeId>]) -> u64 {
    let mut sum = 0;

    for targets in adjacency {
        for neighbor in targets {
            sum += u64::from(neighbor.as_u32());
        }
    }

    sum
}

// Same scan against petgraph. We sum NodeIndex values so the benchmark performs
// observable work comparable to the PalmCDS and Vec baselines.
fn scan_petgraph_neighbors(graph: &DiGraph<(), ()>) -> u64 {
    let mut sum = 0;

    for source in graph.node_indices() {
        for neighbor in graph.neighbors_directed(source, Direction::Outgoing) {
            sum += neighbor.index() as u64;
        }
    }

    sum
}

// Baseline BFS implemented with the same visited-on-enqueue behavior as
// PalmCDS's Bfs iterator, so traversal semantics are comparable.
fn bfs_adjacency(adjacency: &[Vec<NodeId>], start: NodeId) -> u64 {
    let mut visited = vec![false; adjacency.len()];
    let mut queue = VecDeque::new();
    let mut sum = 0;

    visited[start.index()] = true;
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        sum += u64::from(node.as_u32());

        for neighbor in &adjacency[node.index()] {
            let index = neighbor.index();
            if !visited[index] {
                visited[index] = true;
                queue.push_back(*neighbor);
            }
        }
    }

    sum
}

fn bfs_petgraph(graph: &DiGraph<(), ()>, start: NodeIndex) -> u64 {
    let mut visited = vec![false; graph.node_count()];
    let mut queue = VecDeque::new();
    let mut sum = 0;

    visited[start.index()] = true;
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        sum += node.index() as u64;

        for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
            let index = neighbor.index();
            if !visited[index] {
                visited[index] = true;
                queue.push_back(neighbor);
            }
        }
    }

    sum
}

fn traversal_benchmarks(c: &mut Criterion) {
    for shape in SHAPES {
        // Build inputs once per benchmark group so these measurements focus on
        // traversal cost rather than setup cost.
        let graph = build_graph(shape);
        let reordered_graph = build_reordered_graph(shape);
        let adjacency = build_adjacency_list(shape);
        let petgraph = build_petgraph(shape);
        let mut group = c.benchmark_group(format!("traversal/{}", shape.name()));

        group.bench_function("palmcds full neighbor scan", |b| {
            b.iter(|| black_box(scan_graph_neighbors(black_box(&graph))))
        });

        group.bench_function("palmcds reordered full neighbor scan", |b| {
            b.iter(|| black_box(scan_graph_neighbors(black_box(&reordered_graph))))
        });

        group.bench_function("vec adjacency full neighbor scan", |b| {
            b.iter(|| black_box(scan_adjacency_neighbors(black_box(&adjacency))))
        });

        group.bench_function("petgraph full neighbor scan", |b| {
            b.iter(|| black_box(scan_petgraph_neighbors(black_box(&petgraph))))
        });

        group.bench_function("palmcds bfs", |b| {
            b.iter(|| {
                let sum: u64 = graph
                    .bfs(NodeId::new(0))
                    .unwrap()
                    .map(|node| u64::from(node.as_u32()))
                    .sum();
                black_box(sum)
            })
        });

        group.bench_function("palmcds reordered bfs", |b| {
            b.iter(|| {
                let sum: u64 = reordered_graph
                    .bfs(NodeId::new(0))
                    .unwrap()
                    .map(|node| u64::from(node.as_u32()))
                    .sum();
                black_box(sum)
            })
        });

        group.bench_function("vec adjacency bfs", |b| {
            b.iter(|| black_box(bfs_adjacency(black_box(&adjacency), NodeId::new(0))))
        });

        group.bench_function("petgraph bfs", |b| {
            b.iter(|| black_box(bfs_petgraph(black_box(&petgraph), NodeIndex::new(0))))
        });

        group.finish();
    }
}

fn build_benchmarks(c: &mut Criterion) {
    for shape in SHAPES {
        let mut group = c.benchmark_group(format!("build/{}", shape.name()));

        // These benchmarks intentionally include construction and compaction.
        // The reordered variant also includes the BFS relabeling pass.
        group.bench_function("palmcds build", |b| {
            b.iter(|| black_box(build_graph(shape)))
        });

        group.bench_function("palmcds build reordered", |b| {
            b.iter(|| black_box(build_reordered_graph(shape)))
        });

        group.finish();
    }
}

fn footprint_benchmarks(c: &mut Criterion) {
    for shape in SHAPES {
        let graph = build_graph(shape);
        let reordered_graph = build_reordered_graph(shape);
        let adjacency = build_adjacency_list(shape);
        let mut group = c.benchmark_group(format!("footprint/{}", shape.name()));

        // These benchmarks keep footprint calculations visible in Criterion
        // output and prevent the comparison helpers from bit-rotting as the
        // layouts evolve.
        group.bench_function("palmcds footprint bytes", |b| {
            b.iter(|| black_box(black_box(&graph).total_storage_bytes()))
        });

        group.bench_function("palmcds reordered footprint bytes", |b| {
            b.iter(|| black_box(black_box(&reordered_graph).total_storage_bytes()))
        });

        group.bench_function("vec adjacency footprint bytes", |b| {
            b.iter(|| black_box(adjacency_storage_bytes(black_box(&adjacency))))
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    traversal_benchmarks,
    build_benchmarks,
    footprint_benchmarks
);
criterion_main!(benches);
