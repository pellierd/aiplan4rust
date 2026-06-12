use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// Top-level error boundary that unifies all inertia-related sub-errors.
///
/// This enum acts as the main public interface for errors in the inertia module,
/// automatically wrapping errors from submodules via `#[from]` annotations.
#[derive(Error, Debug)]
pub enum InertiaError {
    /// Errors originating from the evaluation phase.
    #[error("Evaluator error: {0}")]
    Evaluator(#[from] InertiaEvaluatorError),

    /// Errors originating from the storage or table construction phase.
    #[error("Table error: {0}")]
    Table(#[from] InertiaTableError),
}

// Permet de propager la trace d'erreur globale si ton framework l'exige
impl Traceable for InertiaError {}
