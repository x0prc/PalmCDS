use criterion::{Criterion, black_box, criterion_group, criterion_main};
use palmcds::{Graph, GraphBuilder, NodeId, Reorder};
use std::collections::VecDeque;
use std::mem::size_of;

const NODE_COUNT: usize = 10_000;
const FANOUT: usize = 4;

// Build a fixed-fanout directed ring. Each node points to the next FANOUT
// nodes, wrapping at the end, so BFS from node 0 reaches the whole graph.
fn build_graph() -> Graph<(), ()> {
    let mut builder = GraphBuilder::with_capacity(NODE_COUNT, NODE_COUNT * FANOUT);
    let nodes: Vec<_> = (0..NODE_COUNT)
        .map(|_| builder.add_node(()).unwrap())
        .collect();

    for source in 0..NODE_COUNT {
        for offset in 1..=FANOUT {
            let target = (source + offset) % NODE_COUNT;
            builder.add_edge(nodes[source], nodes[target], ()).unwrap();
        }
    }

    builder.build().unwrap()
}

// Use the same topology as build_graph, but ask PalmCDS to relabel nodes in BFS
// order during compaction. This isolates the cost and effect of reordering.
fn build_reordered_graph() -> Graph<(), ()> {
    let mut builder = GraphBuilder::with_capacity(NODE_COUNT, NODE_COUNT * FANOUT);
    let nodes: Vec<_> = (0..NODE_COUNT)
        .map(|_| builder.add_node(()).unwrap())
        .collect();

    for source in 0..NODE_COUNT {
        for offset in 1..=FANOUT {
            let target = (source + offset) % NODE_COUNT;
            builder.add_edge(nodes[source], nodes[target], ()).unwrap();
        }
    }

    builder.reorder(Reorder::Bfs { root: nodes[0] });
    builder.build().unwrap()
}

// Simple adjacency-list baseline. This is not meant to be a full competing
// graph library; it is the common Vec<Vec<NodeId>> shape many users start with.
fn build_adjacency_list() -> Vec<Vec<NodeId>> {
    let mut adjacency = (0..NODE_COUNT)
        .map(|_| Vec::with_capacity(FANOUT))
        .collect::<Vec<_>>();

    for (source, targets) in adjacency.iter_mut().enumerate() {
        for offset in 1..=FANOUT {
            let target = (source + offset) % NODE_COUNT;
            targets.push(NodeId::new(target as u32));
        }
    }

    adjacency
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

fn traversal_benchmarks(c: &mut Criterion) {
    // Build inputs once per benchmark group so these measurements focus on
    // traversal cost rather than setup cost.
    let graph = build_graph();
    let reordered_graph = build_reordered_graph();
    let adjacency = build_adjacency_list();

    c.bench_function("palmcds full neighbor scan", |b| {
        b.iter(|| black_box(scan_graph_neighbors(black_box(&graph))))
    });

    c.bench_function("palmcds reordered full neighbor scan", |b| {
        b.iter(|| black_box(scan_graph_neighbors(black_box(&reordered_graph))))
    });

    c.bench_function("vec adjacency full neighbor scan", |b| {
        b.iter(|| black_box(scan_adjacency_neighbors(black_box(&adjacency))))
    });

    c.bench_function("palmcds bfs", |b| {
        b.iter(|| {
            let sum: u64 = graph
                .bfs(NodeId::new(0))
                .unwrap()
                .map(|node| u64::from(node.as_u32()))
                .sum();
            black_box(sum)
        })
    });

    c.bench_function("palmcds reordered bfs", |b| {
        b.iter(|| {
            let sum: u64 = reordered_graph
                .bfs(NodeId::new(0))
                .unwrap()
                .map(|node| u64::from(node.as_u32()))
                .sum();
            black_box(sum)
        })
    });

    c.bench_function("vec adjacency bfs", |b| {
        b.iter(|| black_box(bfs_adjacency(black_box(&adjacency), NodeId::new(0))))
    });
}

fn build_benchmarks(c: &mut Criterion) {
    // These benchmarks intentionally include construction and compaction. The
    // reordered variant also includes the BFS relabeling pass.
    c.bench_function("palmcds build", |b| b.iter(|| black_box(build_graph())));

    c.bench_function("palmcds build reordered", |b| {
        b.iter(|| black_box(build_reordered_graph()))
    });
}

fn footprint_benchmarks(c: &mut Criterion) {
    let graph = build_graph();
    let reordered_graph = build_reordered_graph();
    let adjacency = build_adjacency_list();

    // These benchmarks keep footprint calculations visible in Criterion output
    // and prevent the comparison helpers from bit-rotting as the layouts evolve.
    c.bench_function("palmcds footprint bytes", |b| {
        b.iter(|| black_box(black_box(&graph).total_storage_bytes()))
    });

    c.bench_function("palmcds reordered footprint bytes", |b| {
        b.iter(|| black_box(black_box(&reordered_graph).total_storage_bytes()))
    });

    c.bench_function("vec adjacency footprint bytes", |b| {
        b.iter(|| black_box(adjacency_storage_bytes(black_box(&adjacency))))
    });
}

criterion_group!(
    benches,
    traversal_benchmarks,
    build_benchmarks,
    footprint_benchmarks
);
criterion_main!(benches);
