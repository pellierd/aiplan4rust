//! Provides a preorder (depth-first) iterator over an `ArenaAst` structure,
//! yielding the node index along with a reference to the node.
//!
//! The iterator traverses the tree in preorder, meaning it visits
//! the node itself before visiting its children, from left to right.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, PreorderIterWithIndex};
//!
//! let mut arena = ArenaAst::new();
//! let root = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//! let child = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//! arena.get_node_mut(root).unwrap().add_child(child);
//!
//! let iter = PreorderIterWithIndex::new(&arena, root);
//! for (idx, node) in iter {
//!     println!("Node index: {}, kind: {:?}", idx, node.kind());
//! }
//! ```
//!
//! # Implementation details
//!
//! This iterator uses a stack of node indices to track traversal order.
//! It visits each node before its children, pushing children in reverse order
//! to the stack to maintain left-to-right traversal order.

use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};

pub struct PreorderIterWithIndex<'a> {
    arena: &'a ArenaAst,
    stack: Vec<usize>,
}

impl<'a> PreorderIterWithIndex<'a> {
    /// Creates a new preorder iterator starting from the given root node index.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena containing the AST nodes.
    /// * `root` - The index of the root node from where traversal begins.
    ///
    /// # Returns
    ///
    /// A `PreorderIterWithIndex` ready to iterate over the subtree rooted at `root`.
    pub fn new(arena: &'a ArenaAst, root: usize) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }
}

impl<'a> Iterator for PreorderIterWithIndex<'a> {
    type Item = (usize, &'a ArenaAstNode);

    /// Advances the iterator and returns the next node index and reference in preorder.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        let node_idx = self.stack.pop()?;
        let node = self.arena.get(node_idx)?;

        // Push children in reverse order so that leftmost child is processed first
        for &child_idx in node.children().iter().rev() {
            self.stack.push(child_idx);
        }

        Some((node_idx, node))
    }
}
