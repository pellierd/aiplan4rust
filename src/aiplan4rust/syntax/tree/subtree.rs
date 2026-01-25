//! Subtree abstraction for syntax trees.
//!
//! This module defines the [`SyntaxSubtree`] type, a lightweight wrapper that represents
//! a focused view over a node and its parent [`SyntaxTree`]. This abstraction is useful
//! in contexts where operations require access to both a specific node and the tree
//! structure it belongs to—such as analysis, transformation, or conversion logic.
//!
//! # Purpose
//!
//! [`SyntaxSubtree`] enables ergonomic access to a node in context, reducing the need
//! for repetitive `(node, tree)` tuple passing. It is particularly useful when implementing
//! traits like `TryFrom<Subtree<_>>` or writing recursive analyzers over localized parts of the tree.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::tree::{SyntaxTree, SyntaxNode};
//! use aiplan4rust::syntax::tree::subtree::SyntaxSubtree;
//!
//! fn process_subtree<T: SyntaxNode>(sub: SyntaxSubtree<'_, T>) {
//!     println!("Node kind: {:?}", sub.node().kind());
//!     // You can also inspect the tree:
//!     let root = sub.tree().root_node();
//! }
//! ```
//!
//! # See Also
//! - [`SyntaxTree`]: Represents the full abstract syntax tree.
//! - [`SyntaxNode`]: Trait implemented by all nodes within the tree.

use std::fmt;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxTree};

/// A lightweight wrapper representing a subtree within a [`SyntaxTree`],
/// anchored at a specific syntax node.
///
/// This structure is primarily used to conveniently pass around a node and
/// its context (`SyntaxTree`), for example when implementing conversions
/// like `TryFrom<Subtree<_>>` or performing localized analysis.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxSubtree<'a, T: SyntaxNode> {
    /// The node representing the root of the subtree.
    node: &'a T,

    node_id: NodeId,
    /// The full syntax tree containing the node.
    tree: &'a SyntaxTree<T>,
}

impl<'a, T: SyntaxNode> SyntaxSubtree<'a, T> {
    /// Creates a new `SyntaxSubtree` instance.
    ///
    /// # Arguments
    ///
    /// * `node` - A reference to the root node of the subtree.
    /// * `tree` - A reference to the full syntax tree containing the node.
    ///
    /// # Returns
    ///
    /// A new `SyntaxSubtree` structure encapsulating the node and its tree.
    pub fn new(node: &'a T, node_id: NodeId, tree: &'a SyntaxTree<T>) -> Self {
        Self { node, node_id, tree }
    }

    /// Returns a reference to the node this subtree wraps.
    ///
    /// # Returns
    ///
    /// A reference to the root node of this subtree.
    pub fn node(&self) -> &'a T {
        self.node
    }

    /// Returns a reference to the syntax tree that contains this subtree.
    ///
    /// # Returns
    ///
    /// A reference to the full `SyntaxTree` in which the node resides.
    pub fn tree(&self) -> &'a SyntaxTree<T> {
        self.tree
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
}

impl<'a, T: SyntaxNode + fmt::Display> fmt::Display for SyntaxSubtree<'a, T> {
    /// Formats the `SyntaxSubtree` for user-friendly display purposes.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the display representation.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting was successful.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SyntaxSubtree {{")?;
        writeln!(f, "  node: {},", self.node)?;
        writeln!(f, "  tree info:")?;
        writeln!(f, "    total nodes: {}", self.tree.len())?;
        writeln!(f, "    root id: {:?}", self.tree.root_id())?;
        write!(f, "}}")
    }
}

impl<'a, T: SyntaxNode> From<(&'a T, NodeId, &'a SyntaxTree<T>)> for SyntaxSubtree<'a, T> {
    /// Convertit un tuple `(node, node_id, tree)` en un `SyntaxSubtree`.
    fn from(tuple: (&'a T, NodeId, &'a SyntaxTree<T>)) -> Self {
        let (node, node_id, tree) = tuple;
        SyntaxSubtree::new(node, node_id, tree)
    }
}
