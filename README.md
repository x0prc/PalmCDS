# PalmCDS

PalmCDS is a Rust library for cache-conscious data structures. The current focus is an immutable, directed graph stored in compressed sparse row (CSR) form.

CSR keeps nodes in one contiguous arena and stores each node's outgoing edges in a contiguous range. This avoids pointer-heavy graph layouts and is intended for fast scans and locality-friendly traversal.

## Current API

```rust
use palmcds::{Graph, NodeId};

let graph = Graph::from_edges(
    vec!["a", "b", "c"],
    [
        (NodeId::new(0), NodeId::new(1), 10),
        (NodeId::new(0), NodeId::new(2), 20),
    ],
)?;

for edge in graph.edges_from(NodeId::new(0)).unwrap() {
    println!("target={:?}, weight={}", edge.target(), edge.data());
}
# Ok::<(), palmcds::BuildError>(())
```

## Status

Implemented:

- Immutable directed CSR graph
- Compact `u32` `NodeId`
- Generic node and edge payloads
- Zero-allocation outgoing edge iteration
- Basic construction validation

Planned next:

- `GraphBuilder<N, E>`
- Neighbor-only traversal
- BFS and DFS utilities
- Locality-preserving graph reordering
- Benchmarks against common Rust graph layouts
