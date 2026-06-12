// Importe le trait de ton évaluateur (ajuste le chemin selon ton architecture)
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluatorError;
use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BindingError {
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    ExprBuilder(#[from] ExprBuilderError),

    // Nouvelle variante pour encapsuler l'erreur dynamique de l'évaluateur
    #[error("Evaluation error: {0}")]
    Evaluator(Box<dyn ExprEvaluatorError>),
}

impl BindingError {}
