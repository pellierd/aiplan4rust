use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, Arena, ArenaNode};

pub struct ParseContext {
    interner: RefCell<StringInterner>,
    arena: RefCell<Arena<AstNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            arena: RefCell::new(Arena::empty()),
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
    ) -> Result<NodeId, ParserInternalError> {
        let span = Span::new(start, end);

        // 1) Alloue le nœud avec la liste des enfants
        let node = AstNode::new(kind, content, children, span, parent);

        let mut arena = self.arena.borrow_mut();

        // 2) Alloue le nœud dans l'arène -> on obtient le NodeId du parent
        let node_id = arena.alloc(node);

        // 3) Clone la liste des enfants avant de relâcher l'emprunt
        let children_ids: Vec<NodeId> = {
            let stored_node = arena
                .try_node(node_id)?;
            stored_node.children().to_vec()
        };

        // 4) Met à jour les parents des enfants
        for child_id in &children_ids {
            if let Some(child_node) = arena.get_node_mut(*child_id) {
                child_node.set_parent(Some(node_id));
            }
        }
        let node = arena.try_node(node_id)?;
        //println!("Read: {}", node.to_string_with_interner(&self.interner()));
        Ok(node_id)
    }

    pub fn set_root_id(&self, root_id: NodeId) -> Result<(), ParserInternalError> {
        self.arena.borrow_mut().set_root_id(root_id)
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
    pub fn arena(&self) -> std::cell::RefMut<'_, Arena<AstNode>> {
        self.arena.borrow_mut()
    }

    // Prend l'arene et la remplace par une neuve
    pub fn take_arena(&self) -> Arena<AstNode> {
        std::mem::take(&mut *self.arena.borrow_mut())
    }

    /// Alloue un noeud dans l'arène et retourne son NodeId
    pub fn alloc(&self, node: AstNode) -> NodeId {
        self.arena.borrow_mut().alloc(node)
    }

    /// Accès mutable aux erreurs
    pub fn errors(&self) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow_mut()
    }

    /// Retourne true s'il y a au moins une erreur enregistrée.
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    // Prend les erreurs et remplace par vide
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, LexicalError>> {
        std::mem::take(&mut *self.errors.borrow_mut())
    }

    /// Ajoute une erreur
    pub fn push(&self, error: ErrorRecovery<usize, Token, LexicalError>) {
        self.errors.borrow_mut().push(error);
    }

    /// Merges the children of a `TypedList` node (`next`) into another `TypedList` node (`typed_list`).
    ///
    /// This function takes all children of `next`, updates their parent to be `typed_list`,
    /// and appends them to `typed_list`'s children. After the operation, `next` has no children.
    ///
    /// # Arguments
    ///
    /// * `typed_list` - The `NodeId` of the `TypedList` node that will receive the children.
    /// * `next` - The `NodeId` of the `TypedList` node whose children will be moved.
    ///
    /// # Returns
    ///
    /// Returns the `typed_list` `NodeId` on success, or a `ParserInternalError` if any node access fails.
    pub fn merge_typed_list(
        &mut self,
        typed_list: NodeId,
        next: NodeId,
    ) -> Result<NodeId, ParserInternalError> {
        let mut arena = self.arena();

        // 1) Extract the children of `next` BY DRAINING THEM
        let next_children: Vec<NodeId> = {
            let next_node = arena.try_node_mut(next)?;
            std::mem::take(next_node.children_mut())
        };

        // 2) Update their parent to point to `typed_list`
        for child_id in &next_children {
            let child_node = arena.try_node_mut(*child_id)?;
            child_node.set_parent(Some(typed_list));
        }

        // 3) Append them to `typed_list`
        let typed_list_node = arena.try_node_mut(typed_list)?;
        typed_list_node.children_mut().extend(next_children);

        Ok(typed_list)
    }


}
