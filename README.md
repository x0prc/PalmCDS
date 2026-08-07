# PalmCDS

PalmCDS is a Rust library for cache-conscious data structures. The current focus is an immutable, directed graph stored in compressed sparse row (CSR) form.

CSR keeps nodes in one contiguous arena and stores each node's outgoing edges in a contiguous range. This avoids pointer-heavy graph layouts and is intended for fast scans and locality-friendly traversal.

## Current API

```rust
use palmcds::{Graph, Reorder};

let mut builder = Graph::builder();

let a = builder.add_node("a")?;
let b = builder.add_node("b")?;
let c = builder.add_node("c")?;

builder.add_edge(a, b, 10)?;
builder.add_edge(a, c, 20)?;
builder.reorder(Reorder::Bfs { root: a });

let graph = builder.build()?;

for edge in graph.edges_from(a).unwrap() {
    println!("target={:?}, weight={}", edge.target(), edge.data());
}

for neighbor in graph.neighbors(a).unwrap() {
    println!("neighbor={neighbor:?}");
}

let bfs_order: Vec<_> = graph.bfs(a).unwrap().collect();
let dfs_order: Vec<_> = graph.dfs(a).unwrap().collect();

let reordered = graph.reordered(Reorder::Bfs { root: a })?;
# Ok::<(), palmcds::BuildError>(())
```

You can also build directly from node payloads and `(source, target, payload)` edge triples with `Graph::from_edges`.

## Design

PalmCDS separates graph construction from graph traversal:

- `GraphBuilder<N, E>` is mutable and ergonomic. It collects node payloads and directed edges in insertion order.
- `Graph<N, E>` is immutable. `build()` consumes the builder and compacts the graph into CSR layout.
- `NodeId` is a compact `u32` index. Public accessors return `Option` for out-of-bounds IDs.
- Outgoing edge iteration borrows from contiguous edge storage and does not allocate.
- Neighbor iteration yields only target `NodeId`s for algorithms that do not need edge payloads.
- BFS and DFS traversals borrow the graph and keep traversal state outside the graph storage.
- `Reorder::Bfs` relabels nodes into breadth-first order from a root, then appends disconnected nodes in original order.

## Status

Implemented:

- Immutable directed CSR graph
- `GraphBuilder<N, E>`
- Compact `u32` `NodeId`
- Generic node and edge payloads
- Zero-allocation outgoing edge iteration
- Zero-allocation neighbor-only iteration
- BFS and DFS traversal utilities
- BFS-based locality-preserving reordering
- Basic construction validation
- Criterion benchmark suite for traversal and build paths

## Benchmarks

Run benchmarks with:

```bash
cargo bench
```

The current suite measures:

- Full neighbor scans over PalmCDS CSR storage
- Full neighbor scans over a simple `Vec<Vec<NodeId>>` adjacency-list baseline
- BFS over PalmCDS CSR storage
- BFS over the adjacency-list baseline
- Plain graph build time
- BFS-reordered graph build time

Planned next:

- Add `petgraph` comparisons
- Add larger graph shape variants
- Add memory-footprint reporting
