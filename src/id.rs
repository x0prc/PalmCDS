/// Stable identifier for a node in a [`Graph`](crate::Graph).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates a node identifier from a zero-based index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns this identifier as a zero-based `usize` index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns this identifier as its compact `u32` representation.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}
