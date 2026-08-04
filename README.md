# PalmCDS

PalmCDS is a Rust library for cache-conscious data structures. The current focus is an immutable, directed graph stored in compressed sparse row (CSR) form.

CSR keeps nodes in one contiguous arena and stores each node's outgoing edges in a contiguous range. This avoids pointer-heavy graph layouts and is intended for fast scans and locality-friendly traversal.

## Current API

```rust
use palmcds::Graph;

let mut builder = Graph::builder();

let a = builder.add_node("a")?;
let b = builder.add_node("b")?;
let c = builder.add_node("c")?;

builder.add_edge(a, b, 10)?;
builder.add_edge(a, c, 20)?;

let graph = builder.build()?;

for edge in graph.edges_from(a).unwrap() {
    println!("target={:?}, weight={}", edge.target(), edge.data());
}
# Ok::<(), palmcds::BuildError>(())
```

You can also build directly from node payloads and `(source, target, payload)` edge triples with `Graph::from_edges`.

## Design

PalmCDS separates graph construction from graph traversal:

- `GraphBuilder<N, E>` is mutable and ergonomic. It collects node payloads and directed edges in insertion order.
- `Graph<N, E>` is immutable. `build()` consumes the builder and compacts the graph into CSR layout.
- `NodeId` is a compact `u32` index. Public accessors return `Option` for out-of-bounds IDs.
- Outgoing edge iteration borrows from contiguous edge storage and does not allocate.

## Status

Implemented:

- Immutable directed CSR graph
- `GraphBuilder<N, E>`
- Compact `u32` `NodeId`
- Generic node and edge payloads
- Zero-allocation outgoing edge iteration
- Basic construction validation

Planned next:

- Neighbor-only traversal
- BFS and DFS utilities
- Locality-preserving graph reordering
- Benchmarks against common Rust graph layouts
