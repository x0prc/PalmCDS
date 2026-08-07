use criterion::{Criterion, black_box, criterion_group, criterion_main};
use palmcds::{Graph, GraphBuilder, NodeId, Reorder};
use std::collections::VecDeque;

const NODE_COUNT: usize = 10_000;
const FANOUT: usize = 4;

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

fn scan_graph_neighbors(graph: &Graph<(), ()>) -> u64 {
    let mut sum = 0;

    for source in 0..graph.node_count() {
        for neighbor in graph.neighbors(NodeId::new(source as u32)).unwrap() {
            sum += u64::from(neighbor.as_u32());
        }
    }

    sum
}

fn scan_adjacency_neighbors(adjacency: &[Vec<NodeId>]) -> u64 {
    let mut sum = 0;

    for targets in adjacency {
        for neighbor in targets {
            sum += u64::from(neighbor.as_u32());
        }
    }

    sum
}

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
    c.bench_function("palmcds build", |b| b.iter(|| black_box(build_graph())));

    c.bench_function("palmcds build reordered", |b| {
        b.iter(|| black_box(build_reordered_graph()))
    });
}

criterion_group!(benches, traversal_benchmarks, build_benchmarks);
criterion_main!(benches);
