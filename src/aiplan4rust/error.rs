use crate::aiplan4rust::cli::io::artefact::error::ArtefactError;
use crate::aiplan4rust::cli::io::serialization::SerializationError;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::linking::LinkingError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::normalization::validation::WellNormalizedError;
use crate::aiplan4rust::normalization::NormalizationError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::support::interner::InternerError;
use crate::aiplan4rust::syntax::ast::arena::ArenaError;
use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::SyntaxError;
use thiserror::Error;

pub trait Traceable: std::fmt::Debug + std::fmt::Display + Sized {
    #[track_caller]
    fn trace(self) -> Self {
        if log::log_enabled!(log::Level::Debug) {
            let caller = std::panic::Location::caller();
            let bt = std::backtrace::Backtrace::force_capture();

            log::debug!(
                "Trace at {}:{}:{}\n[Content] {}\n[Backtrace]\n{}",
                caller.file(),
                caller.line(),
                caller.column(),
                self,
                bt
            );
        }
        self
    }
}

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
    Grounding(#[from] GroundingError),

    #[error(transparent)]
    Serialization(#[from] SerializationError),

    #[error(transparent)]
    WellNormalize(#[from] WellNormalizedError),

    #[error(transparent)]
    IO(#[from] ArtefactError),
}

impl AiplanError {
    /// Crée une erreur interne avec un message donné.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        AiplanError::InternalError(msg.into()).trace()
    }
}

impl Traceable for AiplanError {}
