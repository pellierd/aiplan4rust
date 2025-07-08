use std::collections::HashMap;
use std::fmt;
use std::process::id;
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::tree::{TreeNode, NodeId};
use crate::aiplan4rust::tree::iter::{PostorderIter, PostorderIterWithIndex, PreorderIter, PreorderIterWithDepth, PreorderIterWithIndex};
use crate::aiplan4rust::tree::node_ref::{NodeRef, NodeRefMut};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::semantic::{AstArenaNode, symbol::SymbolRef};
use crate::aiplan4rust::syntax::ast::{Ast, AstNode};
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::lang::Ident;

/// A flat arena-based tree structure for storing nodes of type `T`.
///
/// The nodes are stored in a `Vec<T>`, and each node must implement the [`TreeNode`] trait
/// which enables parent/child relationships through indices. This is useful for working
/// with abstract syntax trees and similar structures.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TreeArena<T: TreeNode> {
    pub nodes: Vec<T>,
    root_id: Option<NodeId>,
}

impl<T: TreeNode> TreeArena<T> {
    /// Creates a new, empty tree arena.
    pub fn new() -> Self {
        TreeArena {
            nodes: Vec::new(),
            root_id: Some(NodeId::ROOT_ID)
        }
    }

    pub fn empty() -> Self {
        TreeArena {
            nodes: Vec::new(),
            root_id: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root_id.is_none()
    }

    /// Adds a node into the arena and returns its `NodeId`.
    pub fn alloc(&mut self, node: T) -> NodeId {
        let id = NodeId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Returns a reference to the root node, if it exists.
    pub fn root_node(&self) -> Option<&T> {
        match self.root_id {
            Some(root_id) => self.get_node(root_id),
            None => None,
        }
    }

    /// Returns an immutable `NodeRef` to the root node, if it exists.
    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        match self.root_id {
            Some(root_id) => self.get_node_ref(root_id),
            None => None,
        }
    }

    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), ParserInternalError> {
        if id.as_usize() < self.nodes.len() {
            self.root_id = Some(id);
            Ok(())
        } else {
            Err(ParserInternalError::new("out of bound root id".to_string()))
        }
    }

    /// Returns the parent node of a given node ID, if available.
    pub fn get_parent(&self, id: NodeId) -> Option<&T> {
        self.get_node(id)
            .and_then(|node| node.parent())
            .and_then(|parent_id| self.get_node(parent_id))
    }

    /// Returns an immutable reference to a node by its ID.
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.as_usize())
    }

    /// Returns a `NodeRef` combining the ID and an immutable reference.
    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.nodes.get(id.as_usize()).map(|node| NodeRef::new(id, node))
    }

    /// Returns a mutable reference to a node by its ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Returns a `NodeRefMut` combining the ID and a mutable reference.
    pub fn get_ref_mut(&mut self, id: NodeId) -> Option<NodeRefMut<'_, T>> {
        self.get_node_mut(id).map(|node| NodeRefMut::new(id, node))
    }

    /// Attempts to retrieve a node or returns a `ParserInternalError` if not found.
    pub fn try_node(&self, id: NodeId) -> Result<&T, ParserInternalError> {
        self.get_node(id).ok_or_else(|| {
            ParserInternalError::new(format!("Node with id {} not found", id))
        })
    }

    /// Attempts to retrieve a `NodeRef` or returns an error.
    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, ParserInternalError> {
        let node = self.try_node(id)?;
        Ok(NodeRef::new(id, node))
    }

    /// Attempts to retrieve a mutable node reference or returns an error.
    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, ParserInternalError> {
        self.get_node_mut(id).ok_or_else(|| {
            ParserInternalError::new(format!("Node with id {} not found", id))
        })
    }

    /// Attempts to retrieve a mutable `NodeRefMut` or returns an error.
    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, ParserInternalError> {
        let node = self.try_node_mut(id)?;
        Ok(NodeRefMut::new(id, node))
    }

    /// Attempts to retrieve a `SymbolRef` from a node.
    pub fn try_symbol_ref(&self, id: NodeId) -> Result<SymbolRef, ParserInternalError> {
        let node = self.try_node(id)?;
        node.try_symbol_ref()
    }

    /// Returns the total number of nodes in the arena.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns a preorder iterator starting from the root.
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        match self.root_id {
            Some(root) => PreorderIter::new(self, root),
            None => PreorderIter::empty(self),
        }
    }

    /// Returns a preorder iterator from a specific node.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        PreorderIter::new(self, root)
    }

    /// Returns a preorder iterator with indices from the root.
    pub fn preorder_with_index(&self) -> PreorderIterWithIndex<'_, T> {
        match self.root_id {
            Some(root) => PreorderIterWithIndex::new(self, root),
            None => PreorderIterWithIndex::empty(self),
        }
    }

    /// Returns a preorder iterator with depth from the root.
    pub fn preorder_with_depth(&self) -> PreorderIterWithDepth<'_, T> {
        match self.root_id {
            Some(root) => PreorderIterWithDepth::new(self, root),
            None => PreorderIterWithDepth::empty(self),
        }
    }

    /// Returns a postorder iterator from the root.
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        match self.root_id {
            Some(root) => PostorderIter::new(self, root),
            None => PostorderIter::empty(self),
        }
    }

    /// Returns a postorder iterator from a specific node.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        PostorderIter::new(self, root)
    }

    /// Returns a postorder iterator with indices from the root.
    pub fn postorder_with_index(&self) -> PostorderIterWithIndex<'_, T> {
        match self.root_id {
            Some(root) => PostorderIterWithIndex::new(self, root),
            None => PostorderIterWithIndex::empty(self),
        }
    }

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if !self.is_empty() {
            self.remap_idents_from(self.root_id.unwrap(), map);
        }
    }

    /// Remaps identifiers starting from a specific node (subtree).
    pub fn remap_idents_from(&mut self, id: NodeId, map: &HashMap<Ident, Ident>) {
        let mut stack = vec![id];

        while let Some(current_id) = stack.pop() {
            if let Some(node) = self.get_node_mut(current_id) {
                node.remap_idents(map);

                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }
    }

    /// Computes the total number of nodes in a subtree.
    pub fn size(&self, root: NodeId) -> usize {
        let mut count = 0;
        let mut stack = vec![root];

        while let Some(node_id) = stack.pop() {
            count += 1;
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }

        count
    }

    /// Computes the depth of a subtree (max path from root to leaf).
    pub fn depth(&self, root: NodeId) -> usize {
        let mut max_depth = 0;
        let mut stack = vec![(root, 1)];

        while let Some((node_id, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push((child_id, depth + 1));
                }
            }
        }

        max_depth
    }
}

impl<T> fmt::Display for TreeArena<T>
where
    T: TreeNode + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_node<T: TreeNode + fmt::Display>(
            arena: &TreeArena<T>,
            f: &mut fmt::Formatter<'_>,
            node: &T,
            node_index: usize,
            indent: usize,
            is_last: bool,
        ) -> fmt::Result {
            for _ in 0..indent {
                write!(f, "  ")?;
            }
            writeln!(f, "Node #{}: {}", node_index, node)?;

            let children = node.children();
            for child_idx in children.iter() {
                let child = arena.get_node(*child_idx).expect("Child not found");
                fmt_node(
                    arena,
                    f,
                    child,
                    child_idx.as_usize(),
                    indent + 1,
                    false,
                )?;
            }

            for _ in 0..indent {
                write!(f, "  ")?;
            }

            if is_last {
                write!(f, "End Node #{}", node_index)
            } else {
                writeln!(f, "End Node #{}", node_index)
            }
        }

        if self.is_empty() {
            write!(f, "<empty>")
        } else {
            let root = self.root_node().expect("root_node should exist if not empty");
            let root_index = self.root_id.expect("root_id should exist if not empty").as_usize();
            fmt_node(self, f, root, root_index, 0, true)
        }
    }
}


impl<T> DisplayWithInterner for TreeArena<T>
where
    T: TreeNode,
{
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if self.is_empty() {
            write!(f, "<empty>")
        } else {
            let root = self.root_node().expect("root_node should exist if not empty");
            write!(f, "{}", root.to_string_with_interner(self, interner))
        }
    }
}
impl<T> PlanningSyntaxDisplay for TreeArena<T>
where
    T: TreeNode,
{
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        if self.is_empty() {
            // Arène vide, rien à afficher
            Ok(())
        } else if let Some(root) = self.root_node() {
            root.fmt_planning_syntax_with_indent(f, self, interner, indent)
        } else {
            // Si jamais root_id est Some mais le noeud n'existe pas (cas improbable)
            Ok(())
        }
    }
}








impl TreeArena<AstArenaNode> {
    /// Builds a `TreeArena<AstArenaNode>` from an `Ast`.
    pub fn from_ast(ast: &Ast) -> Self {
        let mut arena = TreeArena::<AstArenaNode>::new();
        let root = ast.root();
        Self::add_iterative(&mut arena, root, None);
        arena
    }

    /// Iteratively adds AST nodes into the arena using a stack-based approach.
    fn add_iterative(
        arena: &mut TreeArena<AstArenaNode>,
        root: &AstNode,
        parent_id: Option<NodeId>,
    ) -> NodeId {
        let mut stack = vec![(root, parent_id)];
        let mut node_ids = HashMap::<*const AstNode, NodeId>::new();

        while let Some((node, parent)) = stack.pop() {
            let arena_node = AstArenaNode::new(
                node.kind().clone(),
                node.content().clone(),
                Vec::new(),
                node.span().clone(),
                parent,
            );

            let node_id = arena.alloc(arena_node);
            node_ids.insert(node as *const AstNode, node_id);

            if let Some(pid) = parent {
                arena.nodes[pid.as_usize()].add_child(node_id);
            }

            for child in node.children().iter().rev() {
                stack.push((child.as_ref(), Some(node_id)));
            }
        }

        node_ids[&(root as *const AstNode)]
    }
}
