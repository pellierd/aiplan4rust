use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::syntax::SyntaxError;

#[derive(Debug, Error)]
pub enum AiplanError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    Syntax(#[from] SyntaxError),

    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    #[error("Interner error: {0}")]
    Interner(#[from] InternerError),

    // autres variantes à venir...
}

impl From<AiplanError> for ArenaError {
    fn from(e: AiplanError) -> Self {
        match e {
            AiplanError::Arena(ae) => ae, // déjà un ArenaError, on renvoie tel quel
            AiplanError::Syntax(se) => {
                ArenaError::InternalError(format!("Syntax error wrapped: {}", se))
            }
            AiplanError::InternalError(msg) => ArenaError::InternalError(msg),
            AiplanError::Interner(ie) => {
                ArenaError::InternalError(format!("Interner error wrapped: {}", ie))
            }
            // gérer les autres variantes si tu en ajoutes plus tard
        }
    }
}
