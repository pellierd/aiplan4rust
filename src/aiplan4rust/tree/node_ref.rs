use std::fmt;
use crate::aiplan4rust::tree::{NodeId, ArenaNode};

/// A lightweight, non-owning reference to a node in an tree.
///
/// `NodeRef` associates a node's unique identifier (`NodeId`) with
/// a reference to the node data itself (`&T`). This allows ergonomic
/// access to both the node and its ID without taking ownership,
/// enabling safe, efficient traversal and inspection of nodes in an tree.
///
/// The lifetime `'a` ties the `NodeRef` to the lifetime of the referenced node,
/// preventing dangling references.
///
/// # Examples
///
/// ```rust
/// let node_ref = NodeRef::new(id, &node);
/// println!("Node ID: {:?}", node_ref.id());
/// println!("Node data: {:?}", node_ref.node());
/// ```
#[derive(Debug, Clone)]
pub struct NodeRef<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRef<'a, T> {
    /// Constructs a new `NodeRef` from the node's ID and a reference to the node.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique identifier of the node.
    /// * `node` - A reference to the node instance in the tree.
    ///
    /// # Returns
    ///
    /// A new `NodeRef` encapsulating the ID and reference.
    pub fn new(id: NodeId, node: &'a T) -> Self {
        Self { id, node }
    }

    /// Returns a reference to the underlying node.
    pub fn node(&self) -> &T {
        self.node
    }

    /// Returns the unique identifier associated with this node.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

/// Enables conversion from `NodeRef` to `NodeId` for convenience.
impl<'a, T: ArenaNode + ?Sized> From<NodeRef<'a, T>> for NodeId {
    fn from(node_ref: NodeRef<'a, T>) -> Self {
        node_ref.id()
    }
}

impl<'a, T: ArenaNode + fmt::Display + ?Sized> fmt::Display for NodeRef<'a, T> {
    /// Formats the `NodeRef` for display, showing the ID and the node's display output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}

/// A mutable, lightweight reference to a node in an tree.
///
/// Similar to `NodeRef`, but allows mutable access to the node.
/// Useful for traversing and modifying nodes safely without taking ownership.
///
/// The lifetime `'a` ensures the mutable reference is valid as long as the tree node exists.
#[derive(Debug)]
pub struct NodeRefMut<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a mut T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRefMut<'a, T> {
    /// Constructs a new mutable node reference from an ID and a mutable node reference.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique identifier of the node.
    /// * `node` - A mutable reference to the node instance.
    ///
    /// # Returns
    ///
    /// A new `NodeRefMut` wrapping the ID and mutable reference.
    pub fn new(id: NodeId, node: &'a mut T) -> Self {
        NodeRefMut { id, node }
    }

    /// Returns the node's unique identifier.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a mutable reference to the underlying node.
    pub fn node_mut(&mut self) -> &mut T {
        self.node
    }
}

impl<'a, T: fmt::Display + ArenaNode> fmt::Display for NodeRefMut<'a, T> {
    /// Formats the mutable node reference for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRefMut {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}
