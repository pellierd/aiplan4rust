//! Module `syntax_tree`
//!
//! Provides a high-level wrapper over an arena-based tree structure tailored for syntax representation.
//!
//! This module defines the [`SyntaxTree`] struct, which supports allocation, traversal,
//! mutation, and identifier remapping of syntax nodes. It is parameterized over types
//! implementing the [`SyntaxNode`] trait and is intended to be used for organizing
//! syntax trees in a structured and type-safe way.
//!
//! The tree is built on top of the low-level [`ArenaTree`] and provides ergonomic access to
//! root nodes, children, parent lookups, tree traversal, and formatting utilities.
//!
//! ## Features
//! - Root and node accessors (mutable and immutable)
//! - Tree traversal (preorder, postorder)
//! - Identifier remapping
//! - Syntax-aware formatting with interner support

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::core::arena::{ArenaTree, NodeId, NodeRef};
use crate::aiplan4rust::core::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::core::arena::node_ref::NodeRefMut;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode};
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// High-level syntax tree built on top of [`ArenaTree`], specialized for syntax node manipulation.
///
/// This structure supports node allocation, hierarchical queries, traversal, and identifier remapping,
/// and is parameterized over types implementing [`SyntaxNode`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SyntaxTree<T: SyntaxNode>
where
    T::Content: SyntaxContent,
{
    arena: ArenaTree<T>,
}

impl<T: SyntaxNode> SyntaxTree<T>
where
    T::Content: SyntaxContent,
{
    /// Creates a new, uninitialized [`SyntaxTree`] instance.
    ///
    /// # Returns
    /// A new `SyntaxTree` with an empty internal arena.
    pub fn new() -> Self {
        SyntaxTree {
            arena: ArenaTree::new(),
        }
    }

    /// Returns an explicitly empty [`SyntaxTree`] instance.
    ///
    /// # Returns
    /// A `SyntaxTree` with an arena that contains no nodes.
    pub fn empty() -> Self {
        SyntaxTree {
            arena: ArenaTree::empty(),
        }
    }

    /// Checks whether the tree is empty.
    ///
    /// # Returns
    /// `true` if the syntax tree contains no nodes, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }

    /// Allocates a new node in the arena.
    ///
    /// # Arguments
    /// * `node` - A syntax node to be inserted into the arena.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc(&mut self, node: T) -> NodeId {
        self.arena.alloc(node)
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

    /// Attempts to extract a [`SymbolRef`] from the node with the given ID.
    ///
    /// # Arguments
    /// * `id` - Node ID to retrieve a symbol from.
    ///
    /// # Returns
    /// `Ok(SymbolRef)` or [`SyntaxTreeError`] if the node is invalid or has no symbol.
    pub fn try_symbol_ref(&self, id: NodeId) -> Result<SymbolRef, SyntaxTreeError> {
        let node = self.try_node(id)?;
        Ok(node.try_symbol_ref()?)
    }

    /// Returns the number of nodes in the tree.
    ///
    /// # Returns
    /// The total count of nodes.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Returns a preorder iterator starting at the root.
    ///
    /// # Returns
    /// A [`PreorderIter`] over all nodes from the root.
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        self.arena.preorder()
    }

    /// Returns a preorder iterator starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PreorderIter`] beginning at `root`.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        self.arena.preorder_from(root)
    }

    /// Returns a postorder iterator starting at the root.
    ///
    /// # Returns
    /// A [`PostorderIter`] over all nodes from the root.
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        self.arena.postorder()
    }

    /// Returns a postorder iterator starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PostorderIter`] beginning at `root`.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        self.arena.postorder_from(root)
    }

    /// Remaps identifiers across the whole syntax tree.
    ///
    /// # Arguments
    /// * `map` - A mapping from old identifiers to new ones.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if !self.is_empty() {
            if let Ok(root_id) = self.arena.try_root_id() {
                self.remap_idents_from(root_id, map);
            }
        }
    }

    /// Remaps identifiers starting from a specific node.
    ///
    /// # Arguments
    /// * `id` - The root of the subtree to apply remapping.
    /// * `map` - A mapping of identifiers to apply.
    pub fn remap_idents_from(&mut self, id: NodeId, map: &HashMap<Ident, Ident>) {
        let mut stack = vec![id];
        while let Some(current_id) = stack.pop() {
            if let Some(node) = self.arena.get_node_mut(current_id) {
                node.remap_idents(map);
                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }
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
}

impl<T> fmt::Display for SyntaxTree<T>
where
    T: SyntaxNode,
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

impl<T> InternerDisplay for SyntaxTree<T>
where
    T: SyntaxNode,
    T::Content: SyntaxContent,
{
    /// Formats the syntax tree using a string interner for resolving identifiers.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the formatted output.
    /// * `interner` - The string interner used to resolve symbol identifiers to their string representations.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting was successful.
    fn fmt_with_interner(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if self.is_empty() {
            write!(f, "<empty>")
        } else {
            let root = self.root_node().expect("root_node should exist if not empty");
            write!(f, "{}", root.to_string_with_interner(self, interner))
        }
    }
}

impl<T> SyntaxDisplay for SyntaxTree<T>
where
    T: SyntaxNode,
    T::Content: SyntaxContent,
{
    /// Formats the syntax tree with indentation and interner-aware resolution of identifiers.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter used to write the formatted output.
    /// * `interner` - A string interner used to resolve identifiers.
    /// * `indent` - The number of spaces to use for indentation in the formatted output.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting was successful.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        if self.is_empty() {
            Ok(()) // Tree is empty; nothing to display.
        } else if let Some(root) = self.root_node() {
            root.fmt_syntax_with_indent(f, self, interner, indent)
        } else {
            Ok(()) // Inconsistent state: root ID exists but no node found.
        }
    }
}
