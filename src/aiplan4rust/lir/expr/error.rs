use thiserror::Error;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Errors specific to the `exp` module, mostly conversion failures.
#[derive(Error, Debug)]
pub enum ExprError {
    /// An error from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),
    
    /// Indicates an unsupported content variant was encountered during conversion.
    #[error("Unsupported content encountered: {0}")]
    UnsupportedContent(String),

    /// Indicates an unsupported kind variant was encountered during conversion.
    #[error("Unsupported kind encountered: {0}")]
    UnsupportedKind(String),

    /// Generic internal error for unexpected states.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ExprError {
    pub fn unsupported_content(msg: impl Into<String>) -> Self {
        ExprError::UnsupportedContent(msg.into())
    }

    pub fn unsupported_kind(msg: impl Into<String>) -> Self {
        ExprError::UnsupportedKind(msg.into())
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        ExprError::InternalError(msg.into())
    }
}
