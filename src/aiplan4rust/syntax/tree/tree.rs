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

/// Wrapper around the low-level Arena, intended as the main entry point for syntax-level operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SyntaxTree<T : SyntaxNode>
    where
        T: SyntaxNode,
        T::Content: SyntaxContent,
{
    arena: ArenaTree<T>,
}

impl<T: SyntaxNode> SyntaxTree<T>
    where
        T: SyntaxNode,
        T::Content: SyntaxContent,
{
    pub fn new() -> Self {
        SyntaxTree {
            arena: ArenaTree::new(),
        }
    }

    pub fn empty() -> Self {
        SyntaxTree {
            arena: ArenaTree::empty(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }

    pub fn alloc(&mut self, node: T) -> NodeId {
        self.arena.alloc(node)
    }

    pub fn root_node(&self) -> Option<&T> {
        self.arena.root_node()
    }

    pub fn try_root(&self) -> Result<&T, SyntaxTreeError> {
        Ok(self.arena.try_root()?)
    }

    pub fn try_root_mut(&mut self) -> Result<&mut T, SyntaxTreeError> {
        Ok(self.arena.try_root_mut()?)
    }

    pub fn root_mut(&mut self) -> Option<&mut T> {
        self.arena.root_mut()
    }

    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        self.arena.root_node_ref()
    }

    pub fn root_id(&self) -> Option<NodeId> {
        self.arena.root_id()
    }

    pub fn try_root_id(&self) -> Result<NodeId, SyntaxTreeError> {
        Ok(self.arena.try_root_id()?)
    }

    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), SyntaxTreeError> {
        Ok(self.arena.set_root_id(id)?)
    }

    pub fn get_parent(&self, id: NodeId) -> Option<&T> {
        self.arena.get_parent(id)
    }

    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.arena.get_node(id)
    }

    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.arena.get_node_ref(id)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.arena.get_node_mut(id)
    }

    pub fn get_ref_mut(&mut self, id: NodeId) -> Option<NodeRefMut<'_, T>> {
        self.arena.get_ref_mut(id)
    }

    pub fn try_node(&self, id: NodeId) -> Result<&T, SyntaxTreeError> {
        Ok(self.arena.try_node(id)?)
    }

    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, SyntaxTreeError> {
        Ok(self.arena.try_node_ref(id)?)
    }

    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, SyntaxTreeError> {
        Ok(self.arena.try_node_mut(id)?)
    }

    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, SyntaxTreeError> {
        Ok(self.arena.try_node_ref_mut(id)?)
    }


    /// Attempts to retrieve a `SymbolRef` from a syntax.
    pub fn try_symbol_ref(&self, id: NodeId) -> Result<SymbolRef, SyntaxTreeError> {
        let node = self.try_node(id)?;
        Ok(node.try_symbol_ref()?) // TODO: handle error properly remove Ok ?
    }



    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Returns a preorder iterator over nodes starting from the root.
    ///
    /// Preorder traversal visits each node before its children, recursively from left to right.
    ///
    /// # Examples
    ///
    /// ```
    /// for node in your_struct.preorder().values() {
    ///     // Process nodes in preorder starting from the root
    /// }
    /// ```
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        self.arena.preorder()
    }

    /// Returns a preorder iterator over nodes starting from the specified node.
    ///
    /// This allows traversing a subtree rooted at `root` in preorder.
    ///
    /// # Arguments
    ///
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Examples
    ///
    /// ```
    /// let root_id = some_node_id;
    /// for node in your_struct.preorder_from(root_id).values() {
    ///     // Process nodes in preorder starting from `root_id`
    /// }
    /// ```
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        self.arena.preorder_from(root)
    }

    /// Returns a postorder iterator over nodes starting from the root.
    ///
    /// Postorder traversal visits children before their parent nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// for node in your_struct.postorder().values() {
    ///     // Process nodes in postorder starting from the root
    /// }
    /// ```
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        self.arena.postorder()
    }

    /// Returns a postorder iterator over nodes starting from the specified node.
    ///
    /// This allows traversing a subtree rooted at `root` in postorder.
    ///
    /// # Arguments
    ///
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Examples
    ///
    /// ```
    /// let root_id = some_node_id;
    /// for node in your_struct.postorder_from(root_id).values() {
    ///     // Process nodes in postorder starting from `root_id`
    /// }
    /// ```
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        self.arena.postorder_from(root)
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if !self.is_empty() {
            self.remap_idents_from(self.arena.try_root_id().unwrap(), map);
        }
    }

    /// Remaps identifiers starting from a specific syntax (subtree).
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

    pub fn size(&self, root: NodeId) -> usize {
        self.arena.size(root)
    }

    pub fn depth(&self, root: NodeId) -> usize {
        self.arena.depth(root)
    }
}

impl<T> fmt::Display for SyntaxTree<T>
where
    T: SyntaxNode,
    T::Content: SyntaxContent,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On délègue à l'affichage de l'arène interne
        self.arena.fmt(f)
    }
}

impl<T> InternerDisplay for SyntaxTree<T>
where
    T: SyntaxNode,
    T::Content: SyntaxContent,
{
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
    fn fmt_syntax_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        if self.is_empty() {
            // Arène vide, rien à afficher
            Ok(())
        } else if let Some(root) = self.root_node() {
            root.fmt_syntax_with_indent(f, self, interner, indent)
        } else {
            // Si jamais root_id est Some mais le noeud n'existe pas (cas improbable)
            Ok(())
        }
    }
}
