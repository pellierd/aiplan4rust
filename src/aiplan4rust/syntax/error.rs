use thiserror::Error;
use lalrpop_util::ParseError;
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::arena::error::ArenaError;

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("Parsing failed: {0}")]
    ParseError(#[from] ParseError<usize, Token, LexicalError>),

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
