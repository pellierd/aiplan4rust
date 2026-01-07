//! Basic arena node implementation for hierarchical tree structures.
//!
//! `BaseNode` is a simple, generic node suitable for representing tree-like
//! structures such as abstract syntax trees (ASTs) or semantic trees. It
//! maintains references to its children and optionally to its parent via
//! [`NodeId`] identifiers, which correspond to nodes stored in an arena.
//!
//! This struct is ideal for use with arena-based data structures where nodes
//! are referenced by IDs and stored efficiently.
//!
//! # Features
//!
//! - Stores a list of child node IDs.
//! - Optionally stores a parent node ID.
//! - Supports adding and modifying children and parent references.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::core::arena::{BaseNode, NodeId};
//!
//! let mut node = BaseNode::new(vec![NodeId::new(1), NodeId::new(2)], None);
//! node.add_child(NodeId::new(3));
//! node.set_parent(Some(NodeId::new(0)));
//!
//! assert_eq!(node.arity(), 3);
//! assert_eq!(node.parent(), Some(NodeId::new(0)));
//! ```

use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::NodeId;

/// A basic arena node used to represent hierarchical structures like abstract syntax trees (ASTs).
///
/// `BaseNode` stores references to its child nodes and optionally to its parent node.
/// This structure is suitable for use in arena-based trees, where each node is identified
/// by a unique [`NodeId`] and stored in a contiguous data structure.
///
/// Common applications include:
/// - Abstract Syntax Trees (ASTs)
/// - Semantic or logical trees
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BaseNode {
    /// The list of child node IDs.
    children: Vec<NodeId>,

    /// The optional parent node ID.
    parent: Option<NodeId>,
}

impl BaseNode {
    /// Creates a new `BaseNode` with the specified children and optional parent.
    ///
    /// # Arguments
    /// - `children`: A vector of child [`NodeId`]s.
    /// - `parent`: An optional [`NodeId`] for the parent node.
    ///
    /// # Returns
    /// A new instance of `BaseNode`.
    ///
    /// # Example
    /// ```
    /// let node = BaseNode::new(vec![NodeId(1), NodeId(2)], Some(NodeId(0)));
    /// ```
    pub fn new(children: Vec<NodeId>, parent: Option<NodeId>) -> Self {
        Self { children, parent }
    }

    /// Returns a shared slice of child node IDs.
    ///
    /// # Example
    /// ```
    /// let child_ids = node.children();
    /// ```
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Returns a mutable reference to the vector of child node IDs.
    ///
    /// This allows for modifying the list of children directly.
    pub fn children_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.children
    }

    /// Replaces the current list of children with a new list.
    ///
    /// # Arguments
    /// - `children`: A vector of [`NodeId`]s to set as the new children.
    ///
    /// # Example
    /// ```
    /// node.set_children(vec![NodeId(3), NodeId(4)]);
    /// ```
    pub fn set_children(&mut self, children: Vec<NodeId>) {
        self.children = children;
    }

    /// Returns the optional parent node ID.
    ///
    /// # Example
    /// ```
    /// if let Some(parent_id) = node.parent() {
    ///     println!("Parent: {:?}", parent_id);
    /// }
    /// ```
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Sets the parent of the current node.
    ///
    /// # Arguments
    /// - `parent`: An optional [`NodeId`] to set as the parent.
    ///
    /// # Example
    /// ```
    /// node.set_parent(Some(NodeId(0)));
    /// ```
    pub fn set_parent(&mut self, parent: Option<NodeId>) {
        self.parent = parent;
    }

    /// Adds a new child to the end of the children list.
    ///
    /// # Arguments
    /// - `child`: The [`NodeId`] of the child node to add.
    ///
    /// # Example
    /// ```
    /// node.add_child(NodeId(5));
    /// ```
    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }
}
