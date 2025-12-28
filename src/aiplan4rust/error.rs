use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::io::error::IOError;
use crate::aiplan4rust::linking::LinkingError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::normalization::NormalizationError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::SyntaxError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::validation::common::WellNormalizedError;

#[derive(Debug, Error)]
pub enum AiplanError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    Syntax(#[from] SyntaxError),

    #[error(transparent)]
    Arena(#[from] ArenaError),

    #[error(transparent)]
    Ast(#[from] AstError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Normalization(#[from] NormalizationError),

    #[error(transparent)]
    Lir(#[from] LirError),

    #[error(transparent)]
    Interner(#[from] InternerError),

    #[error(transparent)]
    Semantic(#[from] SemanticError),

    #[error(transparent)]
    Linking(#[from] LinkingError),

    #[error(transparent)]
    Serialization(#[from] SerializationError),

    #[error(transparent)]
    WellNormalize(#[from] WellNormalizedError),

    #[error(transparent)]
    IO(#[from] IOError),

}

impl AiplanError {
    /// Crée une erreur interne avec un message donné.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        AiplanError::InternalError(msg.into())
    }
}
