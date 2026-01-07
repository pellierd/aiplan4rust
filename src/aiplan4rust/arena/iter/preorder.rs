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
//! // Iterate over nodes in preorder with depth information
//! for (id, depth, node) in arena.preorder_from(root) {
//!     println!("Node {:?} at depth {}: {:?}", id, depth, node);
//! }
//! ```

use crate::aiplan4rust::arena::{ArenaNode, ArenaTree, NodeId, NodeRef};

/// A generic preorder iterator over nodes in an `ArenaTree<T>`.
///
/// This iterator yields `(NodeId, depth, &T)` for each node, starting from
/// the specified root node and traversing parents before their children.
///
/// Preorder traversal is useful for top-down processing of trees,
/// such as syntax analysis or serialization.
///
/// # Example
///
/// ```rust
/// let iter = PreorderIter::new(&arena, root);
/// for (id, depth, node) in iter {
///     println!("Visited node {:?} at depth {}", id, depth);
/// }
/// ```
pub struct PreorderIter<'a, T: ArenaNode> {
    arena: &'a ArenaTree<T>,
    stack: Vec<(NodeId, usize)>, // (node ID, depth)
}

impl<'a, T: ArenaNode> PreorderIter<'a, T> {
    /// Creates a new preorder iterator starting from the given `root`.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena tree to traverse.
    /// * `root` - The starting node ID for traversal.
    ///
    /// # Returns
    ///
    /// A `PreorderIter` that will yield nodes in preorder.
    pub fn new(arena: &'a ArenaTree<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, 0)],
        }
    }

    /// Creates an empty preorder iterator.
    ///
    /// Useful for conditional traversal cases.
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

    /// Iterator that yields only `(NodeId, &T)`, dropping depth information.
    pub fn ids(self) -> impl Iterator<Item = (NodeId, &'a T)> {
        self.map(|(id, _depth, node)| (id, node))
    }
}

impl<'a, T: ArenaNode> Iterator for PreorderIter<'a, T> {
    type Item = (NodeId, usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth) = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Push children in reverse order to maintain left-to-right traversal
        for &child_id in node.children().iter().rev() {
            self.stack.push((child_id, depth + 1));
        }

        Some((id, depth, node))
    }
}
