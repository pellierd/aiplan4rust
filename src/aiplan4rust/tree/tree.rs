//! Module `syntax_tree`
//!
//! Provides a high-level wrapper over an arena-based tree structure tailored for syntax representation.
//!
//! This module defines the [`Tree`] struct, which supports allocation, traversal,
//! mutation, and identifier remapping of syntax nodes. It is parameterized over types
//! implementing the [`Node`] trait and is intended to be used for organizing
//! syntax trees in a structured and typing-safe way.
//!
//! The tree is built on top of the low-level [`ArenaTree`] and provides ergonomic access to
//! root nodes, children, parent lookups, tree traversal, and formatting utilities.
//!
//! ## Features
//! - Root and node accessors (mutable and immutable)
//! - Tree traversal (preorder, postorder)
//! - Identifier remapping
//! - Syntax-aware formatting with interner support

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::aiplan4rust::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::arena::node_ref::NodeRefMut;
use crate::aiplan4rust::arena::{ArenaTree, NodeId, NodeRef};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::{Node, SyntaxContent};

/// High-level syntax tree built on top of [`ArenaTree`], specialized for syntax node manipulation.
///
/// This structure supports node allocation, hierarchical queries, traversal, and identifier remapping,
/// and is parameterized over types implementing [`Node`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Tree<T: Node>
where
    T::Content: SyntaxContent,
{
    arena: ArenaTree<T>,
}

impl<T: Node> Tree<T>
where
    T::Content: SyntaxContent,
{
    /// Creates a new, empty [`Tree`] instance.
    ///
    /// # Returns
    /// A new `SyntaxTree` with an empty internal arena.
    pub fn new() -> Self {
        Tree {
            arena: ArenaTree::new(),
        }
    }

    /// Checks whether the tree is empty.
    ///
    /// # Returns
    /// `true` if the syntax tree contains no nodes, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }

    /// Allocates a new node in the syntax tree.
    ///
    /// # Arguments
    /// * `node` - The syntax node to be inserted.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc(&mut self, node: T) -> NodeId {
        self.arena.alloc(node)
    }

    /// Allocates a new node in the syntax tree and sets it as the root.
    ///
    /// If a root already exists, it is replaced.
    ///
    /// # Arguments
    /// * `node` - The syntax node to be inserted as the root.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly allocated root node.
    pub fn alloc_root(&mut self, node: T) -> NodeId {
        self.arena.alloc_root(node)
    }

    /// Allocates a new node in the syntax tree with specified children.
    ///
    /// Updates the parent reference of each child to point to this node.
    ///
    /// # Arguments
    /// * `node` - The syntax node to be inserted.
    /// * `children` - A vector of [`NodeId`] representing the children of the new node.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc_with_children(&mut self, node: T, children: Vec<NodeId>) -> NodeId {
        self.arena.alloc_with_children(node, children)
    }

    /// Allocates a new node in the syntax tree with specified children and sets it as the root.
    ///
    /// If a root already exists, it is replaced.
    ///
    /// # Arguments
    /// * `node` - The syntax node to be inserted as root.
    /// * `children` - A vector of [`NodeId`] representing the children of the new root node.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly allocated root node.
    pub fn alloc_root_with_children(&mut self, node: T, children: Vec<NodeId>) -> NodeId {
        self.arena.alloc_root_with_children(node, children)
    }

    /// Returns a reference to the root node if available.
    ///
    /// # Returns
    /// `Some(&T)` if the root node exists, otherwise `None`.
    pub fn root_node(&self) -> Option<&T> {
        self.arena.root_node()
    }

    /// Returns a result-wrapped reference to the root node.
    ///
    /// # Returns
    /// `Ok(&T)` if the root exists, otherwise a [`SyntaxTreeError`].
    pub fn try_root(&self) -> Result<&T, SyntaxTreeError> {
        Ok(self.arena.try_root()?)
    }

    /// Returns a mutable reference to the root node.
    ///
    /// # Returns
    /// `Ok(&mut T)` if the root exists, otherwise a [`SyntaxTreeError`].
    pub fn try_root_mut(&mut self) -> Result<&mut T, SyntaxTreeError> {
        Ok(self.arena.try_root_mut()?)
    }

    /// Returns a mutable reference to the root node.
    ///
    /// # Returns
    /// `Some(&mut T)` if the root exists, otherwise `None`.
    pub fn root_mut(&mut self) -> Option<&mut T> {
        self.arena.root_mut()
    }

    /// Returns an immutable reference-wrapped root node.
    ///
    /// # Returns
    /// `Some(NodeRef<T>)` if the root exists, otherwise `None`.
    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        self.arena.root_node_ref()
    }

    /// Returns a result-wrapped node reference for the root node.
    ///
    /// # Returns
    /// `Ok(NodeRef<T>)` or a [`SyntaxTreeError`] if the root doesn't exist.
    pub fn try_root_node_ref(&self) -> Result<NodeRef<'_, T>, SyntaxTreeError> {
        Ok(self.arena.try_root_node_ref()?)
    }

    /// Retrieves the [`NodeId`] of the root node.
    ///
    /// # Returns
    /// `Some(NodeId)` if the root is set, otherwise `None`.
    pub fn root_id(&self) -> Option<NodeId> {
        self.arena.root_id()
    }

    /// Returns the [`NodeId`] of the root node or an error.
    ///
    /// # Returns
    /// `Ok(NodeId)` or a [`SyntaxTreeError`] if the root is unset.
    pub fn try_root_id(&self) -> Result<NodeId, SyntaxTreeError> {
        Ok(self.arena.try_root_id()?)
    }

    /// Sets the root node identifier.
    ///
    /// # Arguments
    /// * `id` - The [`NodeId`] to be used as the root.
    ///
    /// # Returns
    /// `Ok(())` on success or a [`SyntaxTreeError`] if the node doesn't exist.
    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), SyntaxTreeError> {
        Ok(self.arena.set_root_id(id)?)
    }

    /// Retrieves the parent of a node.
    ///
    /// # Arguments
    /// * `id` - The node whose parent is queried.
    ///
    /// # Returns
    /// `Some(&T)` if a parent exists, otherwise `None`.
    pub fn get_parent(&self, id: NodeId) -> Option<&T> {
        self.arena.get_parent(id)
    }

    /// Retrieves a reference to a node by ID.
    ///
    /// # Arguments
    /// * `id` - Identifier of the node to fetch.
    ///
    /// # Returns
    /// `Some(&T)` if the node exists, otherwise `None`.
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.arena.get_node(id)
    }

    /// Retrieves an immutable node reference wrapper by ID.
    ///
    /// # Arguments
    /// * `id` - Node ID to look up.
    ///
    /// # Returns
    /// `Some(NodeRef<T>)` or `None` if the ID is invalid.
    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.arena.get_node_ref(id)
    }

    /// Retrieves a mutable reference to a node by ID.
    ///
    /// # Arguments
    /// * `id` - Identifier of the node.
    ///
    /// # Returns
    /// `Some(&mut T)` if the node exists, otherwise `None`.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.arena.get_node_mut(id)
    }

    /// Retrieves a mutable node reference wrapper by ID.
    ///
    /// # Arguments
    /// * `id` - Node ID to fetch mutably.
    ///
    /// # Returns
    /// `Some(NodeRefMut<T>)` or `None`.
    pub fn get_node_ref_mut(&mut self, id: NodeId) -> Option<NodeRefMut<'_, T>> {
        self.arena.get_ref_mut(id)
    }

    /// Returns a reference to a node or an error.
    ///
    /// # Arguments
    /// * `id` - The ID of the node.
    ///
    /// # Returns
    /// `Ok(&T)` or a [`SyntaxTreeError`] if not found.
    pub fn try_node(&self, id: NodeId) -> Result<&T, SyntaxTreeError> {
        Ok(self.arena.try_node(id)?)
    }

    /// Returns a reference-wrapped node or an error.
    ///
    /// # Arguments
    /// * `id` - Identifier for the node.
    ///
    /// # Returns
    /// `Ok(NodeRef<T>)` or a [`SyntaxTreeError`].
    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, SyntaxTreeError> {
        Ok(self.arena.try_node_ref(id)?)
    }

    /// Returns a mutable reference to a node or an error.
    ///
    /// # Arguments
    /// * `id` - ID of the node to mutate.
    ///
    /// # Returns
    /// `Ok(&mut T)` or a [`SyntaxTreeError`].
    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, SyntaxTreeError> {
        Ok(self.arena.try_node_mut(id)?)
    }

    /// Returns a mutable node reference wrapper or an error.
    ///
    /// # Arguments
    /// * `id` - The ID of the node.
    ///
    /// # Returns
    /// `Ok(NodeRefMut<T>)` or [`SyntaxTreeError`].
    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, SyntaxTreeError> {
        Ok(self.arena.try_node_ref_mut(id)?)
    }

    /// Returns the number of nodes in the tree.
    ///
    /// # Returns
    /// The total count of nodes.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Returns a preorder iter starting at the root.
    ///
    /// # Returns
    /// A [`PreorderIter`] over all nodes from the root.
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        self.arena.preorder()
    }

    /// Returns a preorder iter starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PreorderIter`] beginning at `root`.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        self.arena.preorder_from(root)
    }

    /// Returns a postorder iter starting at the root.
    ///
    /// # Returns
    /// A [`PostorderIter`] over all nodes from the root.
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        self.arena.postorder()
    }

    /// Returns a postorder iter starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PostorderIter`] beginning at `root`.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        self.arena.postorder_from(root)
    }

    /// Returns the total number of nodes in the subtree.
    ///
    /// # Arguments
    /// * `root` - The root node of the subtree.
    ///
    /// # Returns
    /// The number of nodes in the subtree.
    pub fn size(&self, root: NodeId) -> usize {
        self.arena.size(root)
    }

    /// Returns the maximum depth of the subtree.
    ///
    /// # Arguments
    /// * `root` - The root of the subtree.
    ///
    /// # Returns
    /// The depth as number of levels.
    pub fn depth(&self, root: NodeId) -> usize {
        self.arena.depth(root)
    }

    /// Returns `true` if the syntax tree’s underlying arena forms a valid tree.
    ///
    /// This method checks that:
    /// - There is at most one root (or none if empty).
    /// - Every node has at most one parent.
    /// - There are no cycles in the node graph.
    pub fn is_tree(&self) -> bool {
        self.arena.is_tree()
    }

    /// Replaces the kind, content, and children of a node in the tree.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to update.
    /// * `kind` - The new kind to set.
    /// * `content` - The new content to set.
    /// * `children` - The new list of children node IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the `node_id` is invalid.
    pub fn set(
        &mut self,
        node_id: NodeId,
        kind: T::Kind,
        content: T::Content,
        children: Vec<NodeId>,
    ) -> Result<(), SyntaxTreeError> {
        let node = self.try_node_mut(node_id)?; // assuming try_node_mut returns &mut T or error
        node.set_kind(kind);
        *node.content_mut() = content;
        *node.children_mut() = children;
        Ok(())
    }

    /// Moves the kind, content, and children from a source node into a target node.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the node to move data from.
    /// * `target_id` - The ID of the node to move data to.
    ///
    /// # Notes
    ///
    /// This function **takes ownership** of the source node's content and children,
    /// leaving the source node effectively empty. This is useful for in-place
    /// simplifications or transformations without cloning nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// tree.move_node_to(source_id, target_id)?;
    /// ```
    /*pub fn move_to(&mut self, source_id: NodeId, target_id: NodeId) -> Result<(), SyntaxTreeError> {
        let (kind, content, children) = {
            let source = self.try_node_mut(source_id)?;
            (
                source.kind(),
                std::mem::take(source.content_mut()),
                std::mem::take(source.children_mut()),
            )
        };
        self.set(target_id, kind, content, children)?;

        Ok(())
    }*/
    pub fn move_to(&mut self, source_id: NodeId, target_id: NodeId) -> Result<(), SyntaxTreeError> {
        let (kind, content, mut children) = {
            // Note le 'mut' ici
            let source = self.try_node_mut(source_id)?;
            (
                source.kind(),
                std::mem::take(source.content_mut()),
                std::mem::take(source.children_mut()),
            )
        };

        // On met à jour les parents AVANT de perdre la propriété du vecteur
        for &child_id in &children {
            if let Ok(child_node) = self.try_node_mut(child_id) {
                child_node.set_parent(Some(target_id));
            }
        }

        // Maintenant on peut "donner" children à set sans clone
        self.set(target_id, kind, content, children)?;

        Ok(())
    }

    /// Clones a subtree *within the same* `SyntaxTree` and returns
    /// the `NodeId` of the new root node.
    ///
    /// This method performs a deep clone of the subtree rooted at `root_id`,
    /// using [`clone_shallow()`] (or `clone_swallow()`) for each node.
    /// Only the node itself is cloned; children are handled iteratively.
    /// All cloned nodes are allocated in the same tree, with parent/child
    /// relationships rebuilt.
    ///
    /// # Parameters
    /// - `root_id`: the root of the subtree to clone.
    ///
    /// # Returns
    /// - `Ok(NodeId)` pointing to the cloned root node.
    ///
    /// # Errors
    /// - `SyntaxTreeError::NodeNotFound` if `root_id` does not exist.
    pub fn clone_subtree(&mut self, root_id: NodeId) -> Result<NodeId, SyntaxTreeError> {
        // --- Clone root node (shallow, children are empty) ---
        let root_node = self.try_node(root_id)?; // read original root
        let children = root_node.children().to_vec(); // capture children before cloning
        let new_root_id = self.alloc(root_node.clone_shallow()); // use clone_shallow/clone_swallow
        self.try_node_mut(new_root_id)?.set_parent(None); // cloned root has no parent

        // Stack holds nodes to clone: (old_node_id, new_parent_id)
        let mut stack: Vec<(NodeId, NodeId)> = Vec::new();

        // Push root's children onto stack
        for &child_id in children.iter().rev() {
            stack.push((child_id, new_root_id));
        }

        // --- Iterative DFS clone ---
        while let Some((old_id, new_parent_id)) = stack.pop() {
            let old_node = self.try_node(old_id)?; // original node
            let children = old_node.children().to_vec(); // capture children before cloning

            let new_id = self.alloc(old_node.clone_shallow()); // clone node shallowly (clone_swallow)

            // Attach cloned node to its new parent
            self.try_node_mut(new_parent_id)?.add_child(new_id);
            self.try_node_mut(new_id)?.set_parent(Some(new_parent_id));

            // Push children for cloning
            for &child_id in children.iter().rev() {
                stack.push((child_id, new_id));
            }
        }

        Ok(new_root_id)
    }

    /// Returns `true` if the node represents an **atomic formula** or fluent comparison (`FComp`).
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the node is an atomic formula or FComp.
    /// * `Ok(false)` otherwise.
    /// * `Err(ExprError)` if the node ID is invalid.
    pub fn is_atomic_formula(&self, node_id: NodeId) -> Result<bool, SyntaxTreeError> {
        let node = self.try_node(node_id)?;
        Ok(node.is_atomic_formula())
    }

    /// Returns `true` if the node represents a **variable**.
    ///
    /// In the context of a lifted PDDL expression, a variable node acts as
    /// an identifier for a parameter of an action or a quantified variable.
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the node kind is a variable.
    /// * `Ok(false)` otherwise.
    /// * `Err(SyntaxTreeError)` if the node ID is invalid.
    pub fn is_variable(&self, node_id: NodeId) -> Result<bool, SyntaxTreeError> {
        let node = self.try_node(node_id)?;
        Ok(node.is_variable())
    }

    /// Returns `true` if the node is a **temporal specifier** (`AtStart`, `AtEnd`, or `Overall`).
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the node is a temporal specifier.
    /// * `Ok(false)` otherwise.
    /// * `Err(ExprError)` if the node ID is invalid.
    pub fn is_time_specifier(&self, node_id: NodeId) -> Result<bool, SyntaxTreeError> {
        let node = self.try_node(node_id)?;
        Ok(node.is_time_specifier())
    }

    /// Returns `true` if the node represents a **logical operator** (`And`, `Or`, `Not`, `Imply`).
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the node is a logical operator.
    /// * `Ok(false)` otherwise.
    /// * `Err(ExprError)` if the node ID is invalid.
    pub fn is_logic(&self, node_id: NodeId) -> Result<bool, SyntaxTreeError> {
        let node = self.try_node(node_id)?;
        Ok(node.is_logic())
    }

    /// Returns `true` if the node represents a logical negation (`Not`).
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to check.
    ///
    /// # Returns
    /// * `Ok(true)` if the node is a `Not`.
    /// * `Ok(false)` otherwise.
    /// * `Err(ExprError)` if the node ID is invalid.
    pub fn is_not(&self, node_id: NodeId) -> Result<bool, SyntaxTreeError> {
        let node = self.try_node(node_id)?;
        Ok(node.is_not())
    }

    /// Returns `true` if the node designated by `node_id` is a *literal*.
    ///
    /// A literal is defined as:
    /// - an atomic formula (`AtomicFormula` or `FComp`), or
    /// - a negated literal, i.e., a `Not` node whose child is itself a literal.
    ///
    /// This function recursively descends through a chain of `Not` nodes
    /// until it reaches the underlying expression and verifies whether
    /// that expression is an atomic formula.
    ///
    /// # Arguments
    ///
    /// * `node_id` – The ID of the node to test.
    ///
    /// # Returns
    ///
    /// `true` if the node is a literal (possibly negated), `false` otherwise.
    ///
    /// # Examples
    ///
    /// A direct atomic formula:
    /// ```
    /// assert!(tree.is_literal(atom_id));
    /// ```
    ///
    /// A negated atomic formula:
    /// ```
    /// assert!(tree.is_literal(not_id)); // where not_id has atom_id as child
    /// ```
    ///
    /// A logical operator is *not* a literal:
    /// ```
    /// assert!(!tree.is_literal(and_id));
    /// ```
    pub fn is_literal(&self, node_id: NodeId) -> bool {
        if let Ok(node) = self.try_node(node_id) {
            if node.is_not() {
                if let Some(&child_id) = node.children().first() {
                    self.is_literal(child_id)
                } else {
                    false
                }
            } else {
                node.is_atomic_formula()
            }
        } else {
            false
        }
    }
}

impl<T> fmt::Display for Tree<T>
where
    T: Node,
    T::Content: SyntaxContent,
{
    /// Formats the syntax tree using the underlying arena's `Display` implementation.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the formatted output.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting was successful.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.arena.fmt(f)
    }
}
