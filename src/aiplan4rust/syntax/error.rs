use thiserror::Error;
use lalrpop_util::ParseError;
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::context::ParseContextError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("Parsing failed: {0}")]
    ParseError(#[from] ParseError<usize, Token, LexicalError>),

    #[error("Ast error: {0}")]
    Ast(#[from] AstError),

    #[error("ParserContext error: {0}")]
    ParseContext(#[from] ParseContextError),

    #[error("Syntax tree error: {0}")]
    SyntaxTee(#[from] SyntaxTreeError),

    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    #[error("Internal syntax error: {message}")]
    InternalError {
        message: String,
    },
}

impl SyntaxError {

    pub fn as_parse_error(&self) -> Option<&ParseError<usize, Token, LexicalError>> {
        match self {
            SyntaxError::ParseError(ref err) => Some(err),
            _ => None,
        }
    }
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
}

impl From<AiplanError> for SyntaxError {
    fn from(e: AiplanError) -> Self {
        SyntaxError::internal_error(format!("Encapsulated AiplanError: {e}"))
    }
}
