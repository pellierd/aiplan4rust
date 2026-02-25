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

/// A generic preorder iter over nodes in an `ArenaTree<T>`.
///
/// This iter yields `(NodeId, depth, &T)` for each node, starting from
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
// --- 1. MISE À JOUR DE LA STRUCT ---
pub struct PreorderIter<'a, T: ArenaNode> {
    arena: &'a ArenaTree<T>,
    stack: Vec<(NodeId, usize, bool)>,
    skip: bool,
}

impl<'a, T: ArenaNode> PreorderIter<'a, T> {
    pub fn new(arena: &'a ArenaTree<T>, root: NodeId) -> Self {
        Self {
            arena,
            // (ID, profondeur, is_last)
            stack: vec![(root, 0, true)],
            skip: false
        }
    }

    pub fn empty(arena: &'a ArenaTree<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
            skip: false
        }
    }

    pub fn skip_subtree(&mut self) {
        self.skip = true;
    }

    // --- 2. MISE À JOUR DES ADAPTATEURS ---
    // Note le pattern matching (id, _, _, node) pour ignorer le booléen

    pub fn with_id(self) -> impl Iterator<Item = (NodeId, &'a T)> {
        self.map(|(id, _, _, node)| (id, node))
    }

    pub fn with_depth(self) -> impl Iterator<Item = (usize, &'a T)> {
        self.map(|(_, depth, _, node)| (depth, node))
    }

    pub fn node_refs(self) -> impl Iterator<Item = NodeRef<'a, T>> {
        self.map(|(id, _, _, node)| NodeRef::new(id, node))
    }

    pub fn values(self) -> ValuesIter<'a, T> {
        ValuesIter { inner: self }
    }

    pub fn ids(self) -> impl Iterator<Item = (NodeId, &'a T)> {
        self.map(|(id, _, _, node)| (id, node))
    }
}

// --- 3. MISE À JOUR DE L'ITERATOR ---
impl<'a, T: ArenaNode> Iterator for PreorderIter<'a, T> {
    type Item = (NodeId, usize, bool, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth, is_last) = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Si l'utilisateur n'a PAS demandé de skip, on empile les enfants
        if !self.skip {
            let children = node.children();
            let len = children.len();
            for (i, &child_id) in children.iter().enumerate().rev() {
                self.stack.push((child_id, depth + 1, i == len - 1));
            }
        } else {
            // Si on a skipé, on remet le flag à false pour les prochains nœuds frères
            self.skip = false;
        }

        Some((id, depth, is_last, node))
    }
}


pub struct ValuesIter<'a, T: ArenaNode> {
    inner: PreorderIter<'a, T>,
}

impl<'a, T: ArenaNode> ValuesIter<'a, T> {
    // On expose la méthode skip_subtree du parent
    pub fn skip_subtree(&mut self) {
        self.inner.skip_subtree();
    }
}

impl<'a, T: ArenaNode> Iterator for ValuesIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // On ne renvoie que le 4ème élément du tuple
        self.inner.next().map(|(_, _, _, node)| node)
    }
}
