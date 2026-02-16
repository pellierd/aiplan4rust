use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::lang::{AtomSkeletonID, FunctionSkeletonID};
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

#[derive(Error, Debug)]
pub enum InertiaError {

    /// An error related to arena allocation.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from the expr.
    #[error(transparent)]
    Expr(#[from] ExprError),

    #[error("Inertia missing for predicate: {id:?}")]
    MissingPredicateInertia { id: AtomSkeletonID },

    /// L'inertie de la fonction est introuvable dans la table.
    #[error("Inertia missing for function: {id:?}")]
    MissingFunctionInertia { id: FunctionSkeletonID },

    // Arity too high for a predicate
    #[error("Arity too high for indexing: predicate {pred_id:?} has {arity} arguments (max 15)")]
    PredicateArityTooHigh {
        pred_id: AtomSkeletonID,
        arity: usize,
    },

    // Arity too high for a function
    #[error("Arity too high for indexing: function {func_id:?} has {arity} arguments (max 15)")]
    FunctionArityTooHigh {
        func_id: FunctionSkeletonID,
        arity: usize,
    },

}


impl InertiaError {

    /// Creates a new error indicating that inertia information is missing for a predicate.
    ///
    /// This error occurs when a predicate is encountered during the encoding or
    /// analysis phase but has no corresponding entry in the inertia table,
    /// suggesting it was skipped during the initial state or effect scanning pass.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the missing predicate.
    pub fn missing_predicate_inertia(id: AtomSkeletonID) -> Self {
        Self::MissingPredicateInertia { id }
    }

    /// Creates a new error indicating that inertia information is missing for a function.
    ///
    /// This error occurs when a numeric function is encountered but lacks
    /// an entry in the inertia table, preventing the system from determining
    /// if it is a constant or a fluent.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the missing function.
    pub fn missing_function_inertia(id: FunctionSkeletonID) -> Self {
        Self::MissingFunctionInertia { id }
    }

    /// Creates a "predicate arity too high" error
    #[track_caller]
    pub fn predicate_arity_too_high(pred_id: AtomSkeletonID, arity: usize) -> Self {
        Self::PredicateArityTooHigh { pred_id, arity }
    }

    /// Creates a "function arity too high" error
    #[track_caller]
    pub fn function_arity_too_high(func_id: FunctionSkeletonID, arity: usize) -> Self {
        Self::FunctionArityTooHigh { func_id, arity }
    }
}
