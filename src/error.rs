use crate::NodeId;
use core::fmt;

/// Error returned when compact graph construction fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildError {
    /// More than `u32::MAX` nodes were provided.
    TooManyNodes { count: usize },
    /// More than `u32::MAX` edges were provided.
    TooManyEdges { count: usize },
    /// An edge references a node that does not exist in the graph.
    InvalidNodeId { id: NodeId, node_count: usize },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyNodes { count } => {
                write!(f, "graph has {count} nodes, exceeding u32::MAX")
            }
            Self::TooManyEdges { count } => {
                write!(f, "graph has {count} edges, exceeding u32::MAX")
            }
            Self::InvalidNodeId { id, node_count } => {
                write!(
                    f,
                    "node id {} is out of bounds for {node_count} nodes",
                    id.as_u32()
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}

pub(crate) fn validate_node_id(id: NodeId, node_count: usize) -> Result<(), BuildError> {
    if id.index() < node_count {
        Ok(())
    } else {
        Err(BuildError::InvalidNodeId { id, node_count })
    }
}
