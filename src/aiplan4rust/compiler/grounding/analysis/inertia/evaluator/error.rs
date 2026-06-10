use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::error::ValueRegistryError;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, FunctionSkeletonId};
use thiserror::Error;

/// Errors encountered during the construction or evaluation of an `InertiaRegistry`.
///
/// This enum wraps lower-level errors from the syntax tree and expression systems,
/// while providing specific variants for indexing constraints.
#[derive(Error, Debug)]
pub enum InertiaRegistryError {
    /// An error originating from the expression storage system (LIR).
    #[error(transparent)]
    Store(#[from] StorerError),

    #[error(transparent)]
    ValueRegistry(#[from] ValueRegistryError),

    /// An error originating from the underlying syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from the inertia metadata table.
    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    /// The number of arguments in a predicate exceeds the evaluator's indexing capacity.
    #[error("Arity too high for indexing: predicate {pred_id:?} has {arity} arguments")]
    PredicateArityTooHigh {
        /// The identifier of the offending predicate.
        pred_id: AtomSkeletonId,
        /// The actual number of arguments found.
        arity: usize,
    },

    /// The number of arguments in a function exceeds the evaluator's indexing capacity.
    #[error("Arity too high for indexing: function {func_id:?} has {arity} arguments")]
    FunctionArityTooHigh {
        /// The identifier of the offending function.
        func_id: FunctionSkeletonId,
        /// The actual number of arguments found.
        arity: usize,
    },
}

impl InertiaRegistryError {
    /// Creates a new [`InertiaRegistryError::PredicateArityTooHigh`] error.
    ///
    /// This error is raised when the grounding engine encounters a predicate whose
    /// arity exceeds the `max_arity` configured in the evaluator builder.
    ///
    /// # Arguments
    ///
    /// * `pred_id` - The identifier of the predicate.
    /// * `arity` - The detected arity of the predicate.
    #[track_caller]
    pub fn predicate_arity_too_high(pred_id: AtomSkeletonId, arity: usize) -> Self {
        Self::PredicateArityTooHigh { pred_id, arity }
    }

    /// Creates a new [`InertiaRegistryError::FunctionArityTooHigh`] error.
    ///
    /// This error is raised when the grounding engine encounters a function whose
    /// arity exceeds the `max_arity` configured in the evaluator builder.
    ///
    /// # Arguments
    ///
    /// * `func_id` - The identifier of the function.
    /// * `arity` - The detected arity of the function.
    #[track_caller]
    pub fn function_arity_too_high(func_id: FunctionSkeletonId, arity: usize) -> Self {
        Self::FunctionArityTooHigh { func_id, arity }
    }
}
