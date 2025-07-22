//! Lightweight references to arena-managed syntax nodes.
//!
//! This module provides two primary types:
//! - [`NodeRef`]: An immutable, non-owning reference to a syntax node paired with its unique `NodeId`.
//! - [`NodeRefMut`]: A mutable version allowing safe in-place modification of the referenced syntax node.
//!
//! Both types facilitate ergonomic access to syntax nodes stored inside an arena without
//! taking ownership, enabling safe traversal and inspection (or mutation) of tree structures.
//!
//! # Design
//!
//! - Each reference stores a `NodeId` alongside a (mutable or immutable) reference to the node data.
//! - Lifetimes ensure references cannot outlive the arena data they borrow from.
//! - Implements `Debug` and `Display` for easy debugging and user-friendly output.
//! - Supports convenient conversion from `NodeRef` to `NodeId`.
//!
//! # Usage Example
//!
//! ```rust
//! let node_ref = NodeRef::new(id, &syntax_node);
//! println!("Node ID: {:?}", node_ref.id());
//! println!("Node data: {:?}", node_ref.node());
//! ```
//!
//! # Notes
//!
//! These types rely on the trait [`ArenaNode`] which abstracts syntax node behavior in the arena.
//!
//! Mutable access through `NodeRefMut` allows safe modification patterns while preserving arena ownership semantics.

use std::fmt;
use crate::aiplan4rust::core::arena::{NodeId, ArenaNode};

/// A lightweight, non-owning reference to a syntax node within an arena.
///
/// `NodeRef` pairs a syntax node's unique identifier (`NodeId`) with a
/// reference to the syntax data (`&T`). This design enables ergonomic access
/// to both the node’s ID and its data without taking ownership, supporting
/// safe and efficient traversal or inspection of arena nodes.
///
/// The lifetime `'a` ensures that the reference does not outlive the syntax data.
///
/// # Examples
///
/// ```rust
/// let node_ref = NodeRef::new(id, &syntax);
/// println!("Node ID: {:?}", node_ref.id());
/// println!("Node data: {:?}", node_ref.node());
/// ```
#[derive(Debug, Clone)]
pub struct NodeRef<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRef<'a, T> {
    /// Creates a new `NodeRef` from a node ID and a reference to the syntax node.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier for the syntax node.
    /// - `node`: A reference to the syntax node stored in the arena.
    ///
    /// # Returns
    ///
    /// A `NodeRef` encapsulating the ID and reference.
    pub fn new(id: NodeId, node: &'a T) -> Self {
        Self { id, node }
    }

    /// Returns a reference to the underlying syntax node.
    pub fn node(&self) -> &T {
        self.node
    }

    /// Returns the unique identifier associated with this syntax node.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

/// Enables convenient conversion from `NodeRef` to `NodeId`.
impl<'a, T: ArenaNode + ?Sized> From<NodeRef<'a, T>> for NodeId {
    fn from(node_ref: NodeRef<'a, T>) -> Self {
        node_ref.id()
    }
}

impl<'a, T: ArenaNode + fmt::Display + ?Sized> fmt::Display for NodeRef<'a, T> {
    /// Formats the `NodeRef` showing the ID and the syntax node’s display output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, syntax: {} }}", self.id, self.node)
    }
}

/// A mutable, lightweight reference to a syntax node within an arena.
///
/// Like `NodeRef`, but allows mutable access to the syntax node,
/// useful for safe, in-place modification while traversing.
///
/// The lifetime `'a` guarantees the mutable reference remains valid as long
/// as the arena syntax data exists.
#[derive(Debug)]
pub struct NodeRefMut<'a, T: ArenaNode + ?Sized> {
    id: NodeId,
    node: &'a mut T,
}

impl<'a, T: ArenaNode + ?Sized> NodeRefMut<'a, T> {
    /// Creates a new mutable syntax reference from an ID and a mutable reference.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier for the syntax node.
    /// - `node`: A mutable reference to the syntax node in the arena.
    ///
    /// # Returns
    ///
    /// A `NodeRefMut` wrapping the ID and mutable reference.
    pub fn new(id: NodeId, node: &'a mut T) -> Self {
        NodeRefMut { id, node }
    }

    /// Returns the unique identifier of the syntax node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a mutable reference to the underlying syntax node.
    pub fn node_mut(&mut self) -> &mut T {
        self.node
    }
}

impl<'a, T: ArenaNode + fmt::Display> fmt::Display for NodeRefMut<'a, T> {
    /// Formats the mutable syntax reference showing the ID and syntax display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRefMut {{ id: {:?}, syntax: {} }}", self.id, self.node)
    }
}
