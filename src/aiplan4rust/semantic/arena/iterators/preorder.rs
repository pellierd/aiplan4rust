//! Implements a preorder (depth-first) iterator over an `ArenaAst`.
//!
//! # Example
//! ```rust
//! use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, PreorderIter};
//!
//! // Create a new Arena AST
//! let mut arena = ArenaAst::new();
//! let root = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//!
//! // Add some children
//! let child1 = arena.add_node(.., .., vec![]);
//! let child2 = arena.add_node(.., .., vec![]);
//! arena.get_node_mut(root).unwrap().add_child(child1);
//! arena.get_node_mut(root).unwrap().add_child(child2);
//!
//! // Preorder traversal
//! let iter = PreorderIter::new(&arena, root);
//! for node in iter {
//!     println!("{:?}", node.kind());
//! }
//! ```

use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};

/// A preorder (depth-first) iterator over nodes in an `ArenaAst`.
///
/// This iterator starts at a given root node and recursively visits each subtree
/// in left-to-right order using a stack-based approach.
///
/// # Fields
/// - `arena`: A reference to the arena holding the AST nodes.
/// - `stack`: A stack of node indices to visit, emulating recursion.
///
/// # Example
/// See the module-level documentation.
pub struct PreorderIter<'a> {
    arena: &'a ArenaAst,
    stack: Vec<usize>,
}

impl<'a> PreorderIter<'a> {
    /// Creates a new preorder iterator from the given root node index.
    ///
    /// # Arguments
    /// * `arena` - A reference to the arena containing the nodes.
    /// * `root_index` - The index of the root node to start traversal from.
    ///
    /// # Returns
    /// A `PreorderIter` starting at the specified node.
    pub fn new(arena: &'a ArenaAst, root_index: usize) -> Self {
        Self {
            arena,
            stack: vec![root_index],
        }
    }
}

impl<'a> Iterator for PreorderIter<'a> {
    type Item = &'a ArenaAstNode;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.stack.pop()?;
        let node = self.arena.get(index)?;

        // Push children in reverse order so the leftmost child is visited first
        for &child in node.children().iter().rev() {
            self.stack.push(child);
        }

        Some(node)
    }
}
