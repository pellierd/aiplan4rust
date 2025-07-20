use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::{Arena, ArenaNode, NodeId, NodeRef};
use crate::aiplan4rust::arena::error::ArenaError;
use crate::aiplan4rust::arena::iter::{PostorderIter, PostorderIterWithIndex, PreorderIdIter, PreorderIter, PreorderIterWithDepth, PreorderIterWithIndex};
use crate::aiplan4rust::arena::node_ref::NodeRefMut;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::syntax::core::SyntaxNode;
use crate::aiplan4rust::syntax::SyntaxDisplay;

/// Wrapper around the low-level Arena, intended as the main entry point for syntax-level operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SyntaxTree<T : ArenaNode + SyntaxNode> {
    arena: Arena<T>,
}

impl<T: ArenaNode + SyntaxNode> SyntaxTree<T> {
    pub fn new() -> Self {
        SyntaxTree {
            arena: Arena::new(),
        }
    }

    pub fn empty() -> Self {
        SyntaxTree {
            arena: Arena::empty(),
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

    pub fn try_root(&self) -> Result<&T, ArenaError> {
        self.arena.try_root()
    }

    pub fn try_root_mut(&mut self) -> Result<&mut T, ArenaError> {
        self.arena.try_root_mut()
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

    pub fn try_root_id(&self) -> Result<NodeId, ArenaError> {
        self.arena.try_root_id()
    }

    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), ArenaError> {
        self.arena.set_root_id(id)
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

    pub fn try_node(&self, id: NodeId) -> Result<&T, ArenaError> {
        self.arena.try_node(id)
    }

    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, ArenaError> {
        self.arena.try_node_ref(id)
    }

    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, ArenaError> {
        self.arena.try_node_mut(id)
    }

    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, ArenaError> {
        self.arena.try_node_ref_mut(id)
    }


    /// Attempts to retrieve a `SymbolRef` from a syntax.
    pub fn try_symbol_ref(&self, id: NodeId) -> Result<SymbolRef, ArenaError> {
        let node = self.try_node(id)?;
        Ok(node.try_symbol_ref()?) // TODO: handle error properly remove Ok ?
    }



    pub fn len(&self) -> usize {
        self.arena.len()
    }

    pub fn preorder(&self) -> PreorderIter<'_, T> {
        self.arena.preorder()
    }

    pub fn preorder_ids(&self) -> PreorderIdIter<'_, T> {
        self.arena.preorder_ids()
    }

    pub fn preorder_ids_from(&self, root: NodeId) -> PreorderIdIter<'_, T> {
        self.arena.preorder_ids_from(root)
    }

    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        self.arena.preorder_from(root)
    }

    pub fn preorder_with_index(&self) -> PreorderIterWithIndex<'_, T> {
        self.arena.preorder_with_index()
    }

    pub fn preorder_with_depth(&self) -> PreorderIterWithDepth<'_, T> {
        self.arena.preorder_with_depth()
    }

    pub fn postorder(&self) -> PostorderIter<'_, T> {
        self.arena.postorder()
    }

    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        self.arena.postorder_from(root)
    }

    pub fn postorder_with_index(&self) -> PostorderIterWithIndex<'_, T> {
        self.arena.postorder_with_index()
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
    T: ArenaNode + SyntaxNode + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On délègue à l'affichage de l'arène interne
        self.arena.fmt(f)
    }
}

impl<T> InternerDisplay for SyntaxTree<T>
where
    T: ArenaNode + SyntaxNode,
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
    T: ArenaNode + SyntaxNode,
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
