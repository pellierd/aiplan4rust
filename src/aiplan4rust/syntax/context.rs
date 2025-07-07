use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, TreeArena};

pub struct ParseContext {
    interner: RefCell<StringInterner>,
    arena: RefCell<TreeArena<AstArenaNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            arena: RefCell::new(TreeArena::new()),
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
        let node = AstArenaNode::new(kind, content, children.clone(), span, parent);

        let mut arena = self.arena.borrow_mut();

        // 2) Alloue le nœud dans l'arène -> on obtient le NodeId du parent
        let node_id = arena.alloc(node);

        // 3) Met à jour les parents des enfants
        for child_id in &children {
            if let Some(child_node) = arena.get_node_mut(*child_id) {
                child_node.set_parent(Some(node_id));
            }
        }

        node_id
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
}
