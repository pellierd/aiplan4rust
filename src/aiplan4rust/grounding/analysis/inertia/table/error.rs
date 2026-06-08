use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId};
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

/// Errors encountered during inertia analysis or while interacting with the inertia table.
///
/// These errors typically indicate a mismatch between the symbols found in a problem
/// and the pre-computed inertia analysis, or issues during the traversal of the
/// expression old.
#[derive(Error, Debug)]
pub enum InertiaTableError {
    /// An error originating from the expression storage system (LIR).
    #[error(transparent)]
    Store(#[from] StorerError),

    /// An error originating from the syntax tree or expression structures.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Inertia information for a specific predicate is missing from the table.
    ///
    /// This usually happens if the table was built for a different version
    /// of the problem or if a predicate was skipped during analysis.
    #[error("Inertia missing for predicate: {id:?}")]
    MissingPredicateInertia {
        /// The unique identifier of the predicate that was not found.
        id: AtomSkeletonId,
    },

    /// Inertia information for a specific function is missing from the table.
    ///
    /// This usually happens if the table was built for a different version
    /// of the problem or if a function was skipped during analysis.
    #[error("Inertia missing for function: {id:?}")]
    MissingFunctionInertia {
        /// The unique identifier of the function that was not found.
        id: FunctionSkeletonId,
    },
}

impl InertiaTableError {
    /// Creates a new [`InertiaTableError::MissingPredicateInertia`] error.
    ///
    /// This error occurs when a predicate is encountered during the grounding or
    /// encoding phase but has no corresponding entry in the inertia table.
    /// This usually suggests the predicate was not correctly indexed during
    /// the initial state or action effect analysis.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the predicate missing from the metadata.
    ///
    /// # Returns
    ///
    /// A variant of `InertiaTableError` containing the predicate ID.
    pub fn missing_predicate_inertia(id: AtomSkeletonId) -> Self {
        Self::MissingPredicateInertia { id }.trace()
    }

    /// Creates a new [`InertiaTableError::MissingFunctionInertia`] error.
    ///
    /// This error occurs when a numeric function is encountered but lacks
    /// a record in the inertia table. Without this record, the grounding engine
    /// cannot determine if the function is constant (static) or variable (fluent).
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the function missing from the metadata.
    ///
    /// # Returns
    ///
    /// A variant of `InertiaTableError` containing the function ID.
    pub fn missing_function_inertia(id: FunctionSkeletonId) -> Self {
        Self::MissingFunctionInertia { id }.trace()
    }
}

impl Traceable for InertiaTableError {}
