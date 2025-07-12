use std::fmt;
use crate::aiplan4rust::arena::{NodeId, ArenaNode};

/// A lightweight, non-owning reference to a syntax in an arena.
///
/// `NodeRef` associates a syntax's unique identifier (`NodeId`) with
/// a reference to the syntax data itself (`&T`). This allows ergonomic
/// access to both the syntax and its ID without taking ownership,
/// enabling safe, efficient traversal and inspection of nodes in an arena.
///
/// The lifetime `'a` ties the `NodeRef` to the lifetime of the referenced syntax,
/// preventing dangling references.
///
/// # Examples
///
/// ```rust
/// let node_ref = NodeRef::new(id, &syntax);
/// println!("Node ID: {:?}", node_ref.id());
/// println!("Node data: {:?}", node_ref.syntax());
/// ```
#[derive(Debug, Clone)]
pub struct NodeRef<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRef<'a, T> {
    /// Constructs a new `NodeRef` from the syntax's ID and a reference to the syntax.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique identifier of the syntax.
    /// * `syntax` - A reference to the syntax instance in the arena.
    ///
    /// # Returns
    ///
    /// A new `NodeRef` encapsulating the ID and reference.
    pub fn new(id: NodeId, node: &'a T) -> Self {
        Self { id, node }
    }

    /// Returns a reference to the underlying syntax.
    pub fn node(&self) -> &T {
        self.node
    }

    /// Returns the unique identifier associated with this syntax.
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
    /// Formats the `NodeRef` for display, showing the ID and the syntax's display output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, syntax: {} }}", self.id, self.node)
    }
}

/// A mutable, lightweight reference to a syntax in an arena.
///
/// Similar to `NodeRef`, but allows mutable access to the syntax.
/// Useful for traversing and modifying nodes safely without taking ownership.
///
/// The lifetime `'a` ensures the mutable reference is valid as long as the arena syntax exists.
#[derive(Debug)]
pub struct NodeRefMut<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a mut T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRefMut<'a, T> {
    /// Constructs a new mutable syntax reference from an ID and a mutable syntax reference.
    ///
    /// # Parameters
    ///
    /// * `id` - The unique identifier of the syntax.
    /// * `syntax` - A mutable reference to the syntax instance.
    ///
    /// # Returns
    ///
    /// A new `NodeRefMut` wrapping the ID and mutable reference.
    pub fn new(id: NodeId, node: &'a mut T) -> Self {
        NodeRefMut { id, node }
    }

    /// Returns the syntax's unique identifier.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a mutable reference to the underlying syntax.
    pub fn node_mut(&mut self) -> &mut T {
        self.node
    }
}

impl<'a, T: fmt::Display + ArenaNode> fmt::Display for NodeRefMut<'a, T> {
    /// Formats the mutable syntax reference for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRefMut {{ id: {:?}, syntax: {} }}", self.id, self.node)
    }
}
