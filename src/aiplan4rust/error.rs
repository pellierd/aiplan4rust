use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::syntax::SyntaxError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

#[derive(Debug, Error)]
pub enum AiplanError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    Syntax(#[from] SyntaxError),

    #[error(transparent)]
    Arena(#[from] ArenaError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Interner(#[from] InternerError),

    // autres variantes à venir...
}

impl From<AiplanError> for ArenaError {
    fn from(e: AiplanError) -> Self {
        match e {
            AiplanError::Arena(ae) => ae,
            AiplanError::Syntax(se) => {
                ArenaError::InternalError(format!("Syntax error wrapped: {}", se))
            }
            AiplanError::SyntaxTree(ste) => {
                ArenaError::InternalError(format!("SyntaxTree error wrapped: {}", ste))
            }
            AiplanError::InternalError(msg) => ArenaError::InternalError(msg),
            AiplanError::Interner(ie) => {
                ArenaError::InternalError(format!("Interner error wrapped: {}", ie))
            }
            // Ajouter ici le traitement des futures variantes si besoin
        }
    }
}
