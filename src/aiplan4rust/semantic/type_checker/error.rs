use thiserror::Error;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

#[derive(Debug, Error)]
pub enum TypeCheckError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),
}

impl TypeCheckError {
    /// Helper to create an `InternalError` from any displayable message.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        TypeCheckError::InternalError(msg.into())
    }

}
