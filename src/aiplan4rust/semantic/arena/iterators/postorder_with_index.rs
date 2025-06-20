//! Provides a postorder (depth-first) iterator over an `ArenaAst` structure,
//! yielding the node index along with a reference to the node.
//!
//! The iterator traverses the tree in postorder, meaning it visits
//! all children of a node before visiting the node itself.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, PostorderIterWithIndex};
//!
//! let mut arena = ArenaAst::new();
//! let root = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//! let child = arena.add_node(/* kind */ .., /* span */ .., vec![]);
//! arena.get_node_mut(root).unwrap().add_child(child);
//!
//! let iter = PostorderIterWithIndex::new(&arena, root);
//! for (idx, node) in iter {
//!     println!("Node index: {}, kind: {:?}", idx, node.kind());
//! }
//! ```
//!
//! # Implementation details
//!
//! This iterator uses a stack of tuples `(node index, visited flag)` to track traversal order.
//! It visits each node’s children before the node itself.

use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, NodeId};

/// Iterator for postorder traversal of an [`ArenaAst`], yielding node indices and references.
///
/// Visits all nodes so that children are visited before their parent nodes.
/// Each item yielded is a tuple `(usize, &ArenaAstNode)`, where `usize` is the node index.
pub struct PostorderIterWithIndex<'a> {
    arena: &'a ArenaAst,
    stack: Vec<(NodeId, bool)>, // (node index, children visited flag)
}

impl<'a> PostorderIterWithIndex<'a> {
    /// Creates a new postorder iterator starting from the given root node index.
    ///
    /// # Arguments
    /// * `arena` - Reference to the arena containing the AST nodes.
    /// * `root` - The index of the root node from where traversal begins.
    ///
    /// # Returns
    /// A `PostorderIterWithIndex` ready to iterate over the subtree rooted at `root`.
    pub fn new(arena: &'a ArenaAst, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, false)],
        }
    }
}

impl<'a> Iterator for PostorderIterWithIndex<'a> {
    type Item = (NodeId, &'a ArenaAstNode);

    /// Advances the iterator and returns the next node index and reference in postorder.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(node_idx, visited)) = self.stack.last() {
            if !visited {
                // Mark node as visited, will visit children next
                if let Some(top) = self.stack.last_mut() {
                    top.1 = true;
                }

                // Push children in reverse order to visit left to right
                if let Some(node) = self.arena.get_node(node_idx) {
                    for &child_idx in node.children().iter().rev() {
                        self.stack.push((child_idx, false));
                    }
                }
            } else {
                // Children visited, now yield this node
                self.stack.pop();
                if let Some(node) = self.arena.get_node(node_idx) {
                    return Some((node_idx, node));
                }
            }
        }
        None
    }
}
