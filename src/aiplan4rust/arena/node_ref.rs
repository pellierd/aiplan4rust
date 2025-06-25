use std::fmt;
use crate::aiplan4rust::arena::{NodeId, NodeTrait};

/// A lightweight reference to an AST node stored in an arena.
///
/// `NodeRef` holds a reference to a node (`T`) along with its unique identifier (`Id`).
/// This struct allows easy access to the node and its ID without ownership,
/// enabling safe and efficient traversal or manipulation of AST nodes within the arena.
///
/// The lifetime `'a` ensures the `NodeRef` cannot outlive the arena node it references.
#[derive(Copy, Clone, Debug)]
pub struct NodeRef<'a, T: NodeTrait + ?Sized> {
    id: NodeId,
    node: &'a T,
}

impl<'a, T: NodeTrait + ?Sized> NodeRef<'a, T> {
    /// Creates a new `NodeRef` from a node ID and a reference to a node.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier of the node.
    /// - `node`: A reference to the actual node in the arena.
    ///
    /// # Returns
    ///
    /// A new `NodeRef` instance encapsulating the provided ID and node reference.
    pub fn new(id: NodeId, node: &'a T) -> Self {
        Self { id, node }
    }

    /// Returns a reference to the underlying node.
    pub fn node(&self) -> &T {
        self.node
    }

    /// Returns the unique identifier of the node.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

/// Enables conversion from `NodeRef` to `Id` for ergonomic use.
impl<'a, T: NodeTrait + ?Sized> From<NodeRef<'a, T>> for NodeId {
    fn from(node_ref: NodeRef<'a, T>) -> Self {
        node_ref.id()
    }
}

impl<'a, T: NodeTrait + fmt::Display + ?Sized> fmt::Display for NodeRef<'a, T> {
    /// Formats the `NodeRef` for display purposes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}

pub struct NodeRefMut<'a, T: NodeTrait + ?Sized> {
    id: NodeId,
    node: &'a mut T,
}

impl<'a, T: NodeTrait + ?Sized> NodeRefMut<'a, T> {
    pub fn new(id: NodeId, node: &'a mut T) -> Self {
        NodeRefMut { id, node }
    }
    pub fn id(&self) -> NodeId {
        self.id
    }
    pub fn node_mut(&mut self) -> &mut T {
        self.node
    }
}

impl<'a, T: fmt::Display + NodeTrait> fmt::Display for NodeRefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRefMut {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}
