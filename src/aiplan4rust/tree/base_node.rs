//! Module `syntax_base_node`
//!
//! This module defines the generic [`SyntaxBaseNode`] struct, representing a syntax node
//! in an abstract syntax tree (AST).
//!
//! The struct encapsulates a `BaseNode` (managing parent-child relationships),
//! a generic `kind` identifying the node either_type,
//! and a generic `content` holding node-specific data.
//!
//! # Key Features
//!
//! - Construction and modification of syntax nodes with children and parent references.
//! - Access to essential properties: node kind (`kind`), content (`content`), children, and parent.
//! - Integration with an arena system via the [`ArenaNode`] trait.
//! - Supports serialization and deserialization with `serde`.
//!
//! # Typical Usage
//!
//! This either_type is used to represent nodes in syntax trees within a compiler or parser,
//! where each node has a user-defined kind and associated content.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::syntax_base_node::SyntaxBaseNode;
//! use crate::aiplan4rust::syntax::tree::SyntaxContent;
//! use crate::aiplan4rust::core::arena::NodeId;
//!
//! // Assume MyKind and MyContent are defined elsewhere and implement the required traits
//! let kind = MyKind::Expression;
//! let content = MyContent::new(...);
//! let children: Vec<NodeId> = vec![];
//! let parent = None;
//!
//! let node = SyntaxBaseNode::new(kind, content, children, parent);
//! ```
//!
//! [`SyntaxBaseNode`]: struct.SyntaxBaseNode.html
//! [`ArenaNode`]: trait.ArenaNode.html

use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::{ArenaNode, BaseNode, NodeId};
use crate::aiplan4rust::tree::SyntaxContent;

/// Represents a generic syntax node in an abstract syntax tree (AST).
///
/// The `SyntaxBaseNode` struct combines:
/// - A [`BaseNode`] which manages the parent-child relationships within an arena,
/// - A generic `kind` that identifies the specific either_type of the syntax node,
/// - A generic `content` holding the node-specific data.
///
/// This struct is designed to be flexible and reusable across different syntax
/// tree implementations, requiring the `kind` either_type to implement `Copy`, `Debug`, and `Display`,
/// and the `content` either_type to implement the `SyntaxContent` trait.
///
/// # Fields
/// - `base_node`: Manages the hierarchical structure with parent and children node IDs.
/// - `kind`: The either_type or classification of this syntax node.
/// - `content`: Additional information or data associated with this node.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::syntax_base_node::SyntaxBaseNode;
/// use crate::aiplan4rust::syntax::tree::SyntaxContent;
/// use crate::aiplan4rust::core::arena::NodeId;
///
/// // Example kind and content types
/// #[derive(Copy, Clone, Debug, Display)]
/// enum MyKind { Expression, Statement }
///
/// struct MyContent { /* fields omitted */ }
/// impl SyntaxContent for MyContent { /* implementation omitted */ }
///
/// let kind = MyKind::Expression;
/// let content = MyContent { /* init fields */ };
/// let children: Vec<NodeId> = vec![];
/// let parent = None;
///
/// let node = SyntaxBaseNode::new(kind, content, children, parent);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SyntaxBaseNode<K, C>
where
    K: Copy + Debug + Display + PartialEq,
    C: SyntaxContent + PartialEq,
{
    base_node: BaseNode,
    kind: K,
    content: C,
}

impl<K: Copy + Debug + Display + PartialEq, C: SyntaxContent + PartialEq> SyntaxBaseNode<K, C> {
    /// Creates a new `SyntaxBaseNode` with the specified kind, content, children, and optional parent.
    ///
    /// # Arguments
    /// * `kind` - The kind of the syntax node.
    /// * `content` - The content associated with the syntax node.
    /// * `children` - A vector of child node IDs.
    /// * `parent` - An optional parent node ID.
    ///
    /// # Returns
    /// A new instance of `SyntaxBaseNode`.
    pub fn new(kind: K, content: C, children: Vec<NodeId>, parent: Option<NodeId>) -> Self {
        Self {
            base_node: BaseNode::new(children, parent),
            kind,
            content,
        }
    }

    /// Returns the kind of this syntax node.
    ///
    /// # Returns
    /// The kind of the node of either_type `K`.
    pub fn kind(&self) -> K {
        self.kind
    }

    /// Sets the kind of this syntax node.
    ///
    /// # Arguments
    /// * `kind` - The new kind to set.
    pub fn set_kind(&mut self, kind: K) {
        self.kind = kind;
    }

    /// Returns an immutable reference to the content of this syntax node.
    ///
    /// # Returns
    /// A reference to the content of either_type `C`.
    pub fn content(&self) -> &C {
        &self.content
    }

    /// Returns a mutable reference to the content of this syntax node.
    ///
    /// # Returns
    /// A mutable reference to the content of either_type `C`.
    pub fn content_mut(&mut self) -> &mut C {
        &mut self.content
    }

    /// Replaces the content of this syntax node.
    ///
    /// # Arguments
    /// * `content` - The new content to assign.
    pub fn set_content(&mut self, content: C) {
        self.content = content;
    }
}

impl<K: Copy + Debug + Display + PartialEq, C: SyntaxContent + PartialEq> Deref for SyntaxBaseNode<K, C> {
    type Target = BaseNode;

    /// Dereferences to the inner `BaseNode`.
    ///
    /// # Returns
    /// A reference to the internal `BaseNode`.
    fn deref(&self) -> &Self::Target {
        &self.base_node
    }
}

impl<K: Copy + Debug + Display + PartialEq, C: SyntaxContent + PartialEq> DerefMut for SyntaxBaseNode<K, C> {
    /// Dereferences mutably to the inner `BaseNode`.
    ///
    /// # Returns
    /// A mutable reference to the internal `BaseNode`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_node
    }
}

impl<K: Copy + Debug + Display + PartialEq, C: SyntaxContent + PartialEq> ArenaNode for SyntaxBaseNode<K, C> {
    /// Returns the optional parent node ID.
    ///
    /// # Returns
    /// An `Option<NodeId>` representing the parent node if any.
    fn parent(&self) -> Option<NodeId> {
        self.base_node.parent()
    }

    /// Sets the parent node ID.
    ///
    /// # Arguments
    /// * `parent` - An optional `NodeId` to set as the parent.
    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.base_node.set_parent(parent)
    }

    /// Returns a slice of child node IDs.
    ///
    /// # Returns
    /// A slice containing the IDs of all child nodes.
    fn children(&self) -> &[NodeId] {
        self.base_node.children()
    }

    /// Returns a mutable slice of child node IDs.
    ///
    /// # Returns
    ///
    /// A mutable slice containing the IDs of all child nodes of this node,
    /// allowing modification of the children.
    fn children_mut(&mut self) -> &mut Vec<NodeId> {
        self.base_node.children_mut()
    }

    /// Sets the list of child node IDs.
    ///
    /// # Arguments
    /// * `children` - A vector of node IDs to set as children.
    fn set_children(&mut self, children: Vec<NodeId>) {
        self.base_node.set_children(children)
    }

    /// Adds a child node ID to this node.
    ///
    /// # Arguments
    /// * `child` - The node ID to add as a child.
    fn add_child(&mut self, child: NodeId) {
        self.base_node.add_child(child)
    }
}
