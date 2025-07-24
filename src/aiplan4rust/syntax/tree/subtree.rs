use std::fmt;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxTree};

/// A lightweight wrapper representing a subpart (subtree) of a `SyntaxTree`,
/// centered on a given node and its surrounding context.
///
/// Useful for implementing `TryFrom<Subtree<_>>` and reducing tuple boilerplate.
#[derive(Copy, Clone)]
pub struct SyntaxSubtree<'a, T: SyntaxNode> {
    pub node: &'a T,
    pub tree: &'a SyntaxTree<T>,
}

impl<'a, T: SyntaxNode> SyntaxSubtree<'a, T> {
    /// Creates a new `SyntaxSubtree` from a node and its parent syntax tree.
    pub fn new(node: &'a T, tree: &'a SyntaxTree<T>) -> Self {
        Self { node, tree }
    }

    /// Returns a reference to the node.
    pub fn node(&self) -> &'a T {
        self.node
    }

    /// Returns a reference to the full syntax tree.
    pub fn tree(&self) -> &'a SyntaxTree<T> {
        self.tree
    }
}

impl<'a, T: SyntaxNode> fmt::Debug for SyntaxSubtree<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyntaxSubtree")
            .field("node", &self.node)
            .finish()
    }
}

impl<'a, T: SyntaxNode> From<(&'a T, &'a SyntaxTree<T>)> for SyntaxSubtree<'a, T> {
    fn from((node, tree): (&'a T, &'a SyntaxTree<T>)) -> Self {
        SyntaxSubtree::new(node, tree)
    }
}
