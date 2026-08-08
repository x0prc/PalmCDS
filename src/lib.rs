//! Cache-conscious data structures for locality-sensitive workloads.
//!
//! PalmCDS currently provides an immutable directed graph stored in compressed
//! sparse row (CSR) form. Construction is intentionally separated from
//! traversal: [`GraphBuilder`] is mutable and ergonomic, while [`Graph`] is
//! compact and read-only once built.

mod builder;
mod error;
mod graph;
mod id;
mod reorder;
mod traversal;

pub use builder::GraphBuilder;
pub use error::BuildError;
pub use graph::{EdgeRef, Edges, Graph, Neighbors};
pub use id::NodeId;
pub use reorder::Reorder;
pub use traversal::{Bfs, Dfs};
