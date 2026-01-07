//! Iterators for traversing nodes in an `ArenaTree`.
//!
//! This module provides preorder and postorder traversal iterators for
//! trees stored in an arena. These iterators yield nodes along with
//! optional metadata such as node IDs and depth.
//!
//! Traversals are generic over node type_checker `T` implementing the `ArenaNode` trait.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::core::arena::{ArenaTree, NodeId};
//!
//! let arena: ArenaTree<MyNodeType> = ...;
//! let root: NodeId = arena.root_id().unwrap();
//!
//! // Iterate over nodes in postorder with depth information
//! for (id, depth, node) in arena.postorder_from(root) {
//!     println!("Node {:?} at depth {}: {:?}", id, depth, node);
//! }
//! ```

use crate::aiplan4rust::arena::{ArenaNode, ArenaTree, NodeId, NodeRef};

/// A generic postorder iterator over nodes in an `ArenaTree<T>`.
///
/// This iterator yields `(NodeId, depth, &T)` for each node, starting from
/// the specified root node and traversing children before their parent.
///
/// Postorder traversal is useful for bottom-up processing of trees,
/// such as expression evaluation or cleanup operations.
///
/// # Example
///
/// ```rust
/// let iter = PostorderIter::new(&arena, root);
/// for (id, depth, node) in iter {
///     println!("Visited node {:?} at depth {}", id, depth);
/// }
/// ```
pub struct PostorderIter<'a, T: ArenaNode> {
    arena: &'a ArenaTree<T>,
    stack: Vec<(NodeId, usize, bool)>, // (node ID, depth, children_pushed_flag)
}

impl<'a, T: ArenaNode> PostorderIter<'a, T> {
    /// Creates a new postorder iterator starting from the given `root`.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena tree to traverse.
    /// * `root` - The starting node ID for traversal.
    ///
    /// # Returns
    ///
    /// A `PostorderIter` that will yield nodes in postorder.
    pub fn new(arena: &'a ArenaTree<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, 0, false)],
        }
    }

    /// Creates an empty postorder iterator.
    ///
    /// Useful for cases where traversal is conditional.
    pub fn empty(arena: &'a ArenaTree<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }

    /// Transforms this iterator to yield `(NodeId, &T)` tuples,
    /// dropping depth information.
    pub fn with_id(self) -> impl Iterator<Item = (NodeId, &'a T)> {
        self.map(|(id, _, node)| (id, node))
    }

    /// Transforms this iterator to yield `(depth, &T)` tuples,
    /// dropping node ID information.
    pub fn with_depth(self) -> impl Iterator<Item = (usize, &'a T)> {
        self.map(|(_, depth, node)| (depth, node))
    }

    /// Transforms this iterator to yield `NodeRef` structs,
    /// bundling node ID and node reference.
    pub fn node_refs(self) -> impl Iterator<Item = NodeRef<'a, T>> {
        self.map(|(id, _, node)| NodeRef::new(id, node))
    }

    /// Transforms this iterator to yield only node references `&T`,
    /// dropping node ID and depth.
    pub fn values(self) -> impl Iterator<Item = &'a T> {
        self.map(|(_, _, node)| node)
    }

    /// Returns an iterator yielding only `(NodeId, &T)` pairs,
    /// discarding depth information.
    pub fn ids(self) -> impl Iterator<Item = (NodeId, &'a T)> {
        self.map(|(id, _depth, node)| (id, node))
    }
}

impl<'a, T: ArenaNode> Iterator for PostorderIter<'a, T> {
    type Item = (NodeId, usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, depth, visited)) = self.stack.pop() {
            if visited {
                let node = self.arena.get_node(id)?;
                return Some((id, depth, node));
            } else {
                // Push back current node marked as visited
                self.stack.push((id, depth, true));

                // Then push children, reversed to preserve left-to-right order
                let node = self.arena.get_node(id)?;
                for &child_id in node.children().iter().rev() {
                    self.stack.push((child_id, depth + 1, false));
                }
            }
        }
        None
    }
}
