use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, TreeArena};
use crate::Parser;

pub struct ParseContext {
    interner: RefCell<StringInterner>,
    arena: RefCell<TreeArena<AstArenaNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            arena: RefCell::new(TreeArena::empty()),
            errors: RefCell::new(Vec::new()),
        }
    }

    pub fn new_node(
        &self,
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        start: usize,
        end: usize,
        parent: Option<NodeId>,
    ) -> NodeId {
        let span = Span::new(start, end);

        // 1) Alloue le nœud avec la liste des enfants
        let node = AstArenaNode::new(kind, content, children, span, parent);

        let mut arena = self.arena.borrow_mut();

        // 2) Alloue le nœud dans l'arène -> on obtient le NodeId du parent
        let node_id = arena.alloc(node);

        // 3) Clone la liste des enfants avant de relâcher l'emprunt
        let children_ids: Vec<NodeId> = {
            let stored_node = arena
                .get_node(node_id)
                .unwrap(); // ne peut pas échouer juste après alloc
            stored_node.children().to_vec()
        };

        // 4) Met à jour les parents des enfants
        for child_id in &children_ids {
            if let Some(child_node) = arena.get_node_mut(*child_id) {
                child_node.set_parent(Some(node_id));
            }
        }

        node_id
    }

    pub fn set_root_id(&self, root_id: NodeId) {
        self.arena.borrow_mut().set_root_id(root_id).expect("REASON");
    }

    pub fn root_id(&self) -> Option<NodeId> {
        self.arena.borrow().root_id()
    }

    /// Accès mutable au StringInterner
    pub fn interner(&self) -> std::cell::RefMut<'_, StringInterner> {
        self.interner.borrow_mut()
    }

    // Prend l'interner et le remplace par un neuf
    pub fn take_interner(&self) -> StringInterner {
        std::mem::take(&mut *self.interner.borrow_mut())
    }

    /// Interne une chaîne via StringInterner
    pub fn intern(&self, s: String) -> Ident {
        self.interner.borrow_mut().intern(s)
    }

    /// Accès mutable à l'arène
    pub fn arena(&self) -> std::cell::RefMut<'_, TreeArena<AstArenaNode>> {
        self.arena.borrow_mut()
    }

    // Prend l'arene et la remplace par une neuve
    pub fn take_arena(&self) -> TreeArena<AstArenaNode> {
        std::mem::take(&mut *self.arena.borrow_mut())
    }

    /// Alloue un noeud dans l'arène et retourne son NodeId
    pub fn alloc(&self, node: AstArenaNode) -> NodeId {
        self.arena.borrow_mut().alloc(node)
    }

    /// Accès mutable aux erreurs
    pub fn errors(&self) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow_mut()
    }

    // Prend les erreurs et remplace par vide
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, LexicalError>> {
        std::mem::take(&mut *self.errors.borrow_mut())
    }

    /// Ajoute une erreur
    pub fn push(&self, error: ErrorRecovery<usize, Token, LexicalError>) {
        self.errors.borrow_mut().push(error);
    }

    /// Merges the children of the `next` node into the `typed_list` node under a specific condition.
    ///
    /// This method checks if the first child of the `next` node has non-empty children.
    /// If so, it drains all the children from `next`
    /// and appends them as children of the `typed_list` node.
    ///
    /// # Arguments
    ///
    /// * `typed_list` - The target node ID (`NodeId`) in the arena where children will be added.
    /// * `next` - The source node ID (`NodeId`) whose children will be transferred if the condition is met.
    ///
    /// # Behavior
    ///
    /// 1. Immutably borrows the `next` node and checks whether its first child has children.
    /// 2. If the first child has children (i.e., is not a leaf),
    ///    it drains the children from the `next` node.
    /// 3. These drained children are then appended as children to the `typed_list` node.
    ///
    /// # Panics
    ///
    /// This method will panic if either `typed_list` or `next` nodes are not found in the arena.
    ///
    /// # Example
    ///
    /// ```
    /// // Assuming a valid context and NodeIds...
    /// context.merge_typed_list(typed_list_id, next_id);
    /// ```
    ///
    /// This method is designed to be used in syntax tree manipulation,
    /// where `typed_list` and `next` represent nodes within an AST arena.
    ///
    /// # Note
    ///
    /// This function consumes the children of `next` only if the condition is satisfied,
    /// thus mutating the internal state of the involved nodes.
   pub fn merge_typed_list(
        &mut self,
        typed_list: NodeId,
        next: NodeId,
    ) {
        let should_merge = {
            let arena = self.arena();
            let next_node = arena.get_node(next).unwrap();
            if let Some(first_child) = next_node.children().get(0) {
                let first_child_node = arena.get_node(*first_child).unwrap();
                !first_child_node.children().is_empty()
            } else {
                false
            }
        };

        if should_merge {
            let mut arena = self.arena();
            let drained_children = {
                let next_node = arena.get_node_mut(next).unwrap();
                std::mem::take(next_node.children_mut())
            };
            let typed_list_node = arena.get_node_mut(typed_list).unwrap();
            typed_list_node.children_mut().extend(drained_children);
        }
    }

    /*pub fn merge_typed_list(
        &mut self,
        typed_list: NodeId,
        next: NodeId,
    ) -> Result<(), ParserInternalError> {
        let should_merge = {
            let arena = self.arena();
            let next_node = arena.try_node(next)?;
            if let Some(first_child) = next_node.children().get(0) {
                let first_child_node = arena.try_node(*first_child)?;
                !first_child_node.children().is_empty()
            } else {
                false
            }
        };

        if should_merge {
            let mut arena = self.arena();
            let drained_children = {
                let next_node = arena.try_node_mut(next)?;
                std::mem::take(next_node.children_mut())
            };
            let typed_list_node = arena.try_node_mut(typed_list)?;
            typed_list_node.children_mut().extend(drained_children);
        }
        Ok(())
    }*/
}
