use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

#[derive(Debug, Error)]
pub enum LinkingError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SemanticError(#[from] SemanticError),

    #[error(transparent)]
    SemanticCheckError(#[from] SemanticCheckError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    Interner(#[from] InternerError),
}

impl LinkingError {
    /// Helper to create an `InternalError` from any displayable message.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        LinkingError::InternalError(msg.into())
    }

}
