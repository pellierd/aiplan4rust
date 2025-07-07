use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
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
