//! Provides a postorder (depth-first) iterator over an `ArenaAst` structure.
//!
//! The iterator traverses the tree in postorder, meaning it visits
//! all children of a node before visiting the node itself.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, PostorderIter};
//!
//! // Create a new arena and build a simple tree:
//! let mut arena = ArenaAst::new();
//! let root = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//!
//! let child1 = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//! let child2 = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//!
//! arena.get_node_mut(root).unwrap().add_child(child1);
//! arena.get_node_mut(root).unwrap().add_child(child2);
//!
//! // Iterate over nodes in postorder starting from root:
//! let iter = PostorderIter::new(&arena, root);
//! for node in iter {
//!     println!("{:?}", node.kind());
//! }
//! ```
//!
//! # Implementation details
//!
//! The iterator uses an explicit stack to simulate recursion,
//! storing tuples `(node_index, next_child_index)` to keep track of
//! which child to visit next for each node.
//!
//! This design avoids recursion and is efficient in both time and memory.

use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};

pub struct PostorderIter<'a> {
    arena: &'a ArenaAst,
    stack: Vec<(usize, usize)>, // (node_index, next_child_index)
}

impl<'a> PostorderIter<'a> {
    /// Creates a new `PostorderIter` starting from the given `root` node index.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena containing the nodes.
    /// * `root` - The index of the root node where iteration begins.
    ///
    /// # Returns
    ///
    /// A `PostorderIter` instance ready to iterate over the subtree starting at `root`.
    pub fn new(arena: &'a ArenaAst, root: usize) -> Self {
        Self {
            arena,
            stack: vec![(root, 0)],
        }
    }
}

impl<'a> Iterator for PostorderIter<'a> {
    type Item = &'a ArenaAstNode;

    /// Advances the iterator and returns the next node in postorder.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node_idx, child_idx)) = self.stack.pop() {
            let node = self.arena.get_node(node_idx)?;

            if child_idx < node.children().len() {
                // Push current node back with incremented child index to visit next child later
                self.stack.push((node_idx, child_idx + 1));
                // Push the child to visit its subtree first
                self.stack.push((node.children()[child_idx], 0));
            } else {
                // All children visited, now yield the node itself
                return Some(node);
            }
        }
        None
    }
}
