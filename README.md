# PalmCDS

Cache-conscious data structures for locality-sensitive workloads. Currently provides an immutable directed graph stored in compressed sparse row (CSR) form with Structure-of-Arrays (SoA) edge storage.

## Structure

```
src/
  lib.rs         Crate root. Re-exports public API.
  graph.rs       Graph<N, E> — immutable CSR graph with SoA edge storage.
  builder.rs     GraphBuilder<N, E> — mutable builder, compacts into Graph on build.
  id.rs          NodeId — compact u32 graph node identifier.
  traversal.rs   Bfs / Dfs — zero-allocation traversal iterators.
  reorder.rs     Reorder — node ordering strategies (BFS, DFS, RPO, topological, RCM).
  algorithms.rs  Graph algorithms (topological sort, cycle detection, Dijkstra, SCC).
  error.rs       BuildError and validation helpers.
```

## Design

Construction and traversal are separated. `GraphBuilder` collects nodes and edges mutably. Calling `build()` or `from_edges()` compacts everything into an immutable `Graph`. Edge targets and edge payloads are stored in separate contiguous arrays so neighbor scans never pollute L1 cache with irrelevant payload data.

`NodeId` is a `u32` index. All public accessors return `Option` for out-of-bounds IDs.

## Usage

```rust
use palmcds::{Graph, Reorder};

let graph = Graph::from_edges(
    vec!["a", "b", "c"],
    [
        (palmcds::NodeId::new(0), palmcds::NodeId::new(1), 10_u64),
        (palmcds::NodeId::new(0), palmcds::NodeId::new(2), 20),
    ],
).unwrap();

// Iterate edge payloads
for edge in graph.edges_from(palmcds::NodeId::new(0)).unwrap() {
    println!("target={:?} weight={}", edge.target(), edge.data());
}

// Iterate neighbor IDs only
for neighbor in graph.neighbors(palmcds::NodeId::new(0)).unwrap() {
    println!("neighbor={neighbor:?}");
}

// Traversal
let bfs: Vec<_> = graph.bfs(palmcds::NodeId::new(0)).unwrap().collect();
let dfs: Vec<_> = graph.dfs(palmcds::NodeId::new(0)).unwrap().collect();

// Reordering for cache locality
let reordered = graph.reordered(Reorder::Bfs { root: palmcds::NodeId::new(0) }).unwrap();

// Algorithms
let order = graph.topological_sort().unwrap();
let dists = graph.dijkstra(palmcds::NodeId::new(0), |&w| w).unwrap();
```

## Reorder strategies

| Variant | Use case |
|---|---|
| `Bfs { root }` | Level-order locality from a root |
| `Dfs { root }` | Stack-local access patterns |
| `ReversePostOrder { root }` | Dependencies before consumers (DAGs, control flow) |
| `Topological` | Global topological ordering |
| `ReverseCuthillMcKee { root }` | Bandwidth reduction, tight neighbor packing |

## Run

```bash
cargo test
cargo bench
```
