use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::error::ValueRegistryError;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, FunctionSkeletonId};
use thiserror::Error;

/// Errors encountered during the initialization, compilation, or evaluation
/// phase of an [`InertiaEvaluator`].
///
/// This enum encapsulates low-level subsystem errors (LIR store, syntax tree,
/// value registries) and provides domain-specific variants for arity and indexing
/// constraints dictated by the IPP (Inertia Pattern Pruning) analysis pipeline.
#[derive(Error, Debug)]
pub enum InertiaEvaluatorError {
    /// An error originating from the low-level expression storage system (LIR).
    #[error(transparent)]
    Store(#[from] StorerError),

    /// An error originating from the type and object valuation registry.
    #[error(transparent)]
    ValueRegistry(#[from] ValueRegistryError),

    /// An error originating from the underlying PDDL abstract syntax tree (AST) system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from the static analysis inertia metadata table.
    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    /// The number of arguments in a predicate exceeds the evaluator's indexing capacity.
    ///
    /// This occurs during the initial state processing when a predicate's structure
    /// breaches the static bitmask compilation limits or `max_proj` settings.
    #[error("Arity too high for indexing: predicate {pred_id:?} has {arity} arguments")]
    PredicateArityTooHigh {
        /// The identifier of the offending predicate skeleton.
        pred_id: AtomSkeletonId,
        /// The actual number of arguments found in the LIR node.
        arity: usize,
    },

    /// The number of arguments in a numeric or object function exceeds the evaluator's indexing capacity.
    ///
    /// This occurs during the initial state processing when a fluent function's structure
    /// breaches the static bitmask compilation limits or `max_proj` settings.
    #[error("Arity too high for indexing: function {func_id:?} has {arity} arguments")]
    FunctionArityTooHigh {
        /// The identifier of the offending function skeleton.
        func_id: FunctionSkeletonId,
        /// The actual number of arguments found in the LIR node.
        arity: usize,
    },
}

impl InertiaEvaluatorError {
    /// Creates a new [`InertiaEvaluatorError::PredicateArityTooHigh`] error variant.
    ///
    /// This error is raised by the initialization pipeline when the grounding engine
    /// encounters an atomic formula whose logical arity exceeds the `max_arity` or
    /// combinatorial projections configured in the evaluator builder.
    ///
    /// # Arguments
    ///
    /// * `pred_id` - The unique identifier of the target predicate skeleton.
    /// * `arity` - The detected structural arity of the predicate.
    #[inline]
    #[track_caller]
    pub fn predicate_arity_too_high(pred_id: AtomSkeletonId, arity: usize) -> Self {
        Self::PredicateArityTooHigh { pred_id, arity }
    }

    /// Creates a new [`InertiaEvaluatorError::FunctionArityTooHigh`] error variant.
    ///
    /// This error is raised by the initialization pipeline when the grounding engine
    /// encounters a function term (numeric fluent or object fluent) whose logical arity
    /// exceeds the `max_arity` or combinatorial projections configured in the evaluator builder.
    ///
    /// # Arguments
    ///
    /// * `func_id` - The unique identifier of the target function skeleton.
    /// * `arity` - The detected structural arity of the function.
    #[inline]
    #[track_caller]
    pub fn function_arity_too_high(func_id: FunctionSkeletonId, arity: usize) -> Self {
        Self::FunctionArityTooHigh { func_id, arity }
    }
}
