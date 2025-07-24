use thiserror::Error;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

#[derive(Debug, Error)]
pub enum LangError {

    /// An error from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error("Internal Error: {0}")]
    InternalError(String),
}

impl LangError {

    /// Creates an [`InternalError`] with a custom message.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }
}
