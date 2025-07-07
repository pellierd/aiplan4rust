use lalrpop_util::ErrorRecovery;
use crate::aiplan4rust::syntax::lexer::LexicalError;
use crate::aiplan4rust::syntax::lexer::Token;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::AstArenaNode;

use crate::aiplan4rust::tree::TreeArena;
// ou autre bibliothèque d'arène utilisée

pub struct ParseContext<'ctx> {
    interner: &'ctx mut StringInterner,
    errors: &'ctx mut Vec<ErrorRecovery<usize, Token, LexicalError>>,
    arena: &'ctx mut TreeArena<AstArenaNode>,
}

impl<'ctx> ParseContext<'ctx> {
    pub fn new(
        interner: &'ctx mut StringInterner,
        errors: &'ctx mut Vec<ErrorRecovery<usize, Token, LexicalError>>,
        arena: &'ctx mut TreeArena<AstArenaNode>,
    ) -> Self {
        Self {
            interner,
            errors,
            arena,
        }
    }

    pub fn interner(&mut self) -> &mut StringInterner {
        self.interner
    }

    pub fn errors(&mut self) -> &mut Vec<ErrorRecovery<usize, Token, LexicalError>> {
        self.errors
    }

    pub fn arena(&mut self) -> &mut TreeArena<AstArenaNode> {
        self.arena
    }
}
