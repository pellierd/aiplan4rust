use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::expr::{ExprId, ExprKind};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExprOpError {
    #[error(transparent)]
    Store(#[from] StorerError),

    #[error(transparent)]
    ExpBuilder(#[from] ExprBuilderError),

    /// Indicates an illegal nesting of temporal operators (e.g., 'at start' inside 'at end').
    /// This is typically caught during parsing or initial expression building.
    #[error("Illegal Temporal Nesting: Cannot nest temporal operator {nested_kind:?} inside {parent_kind:?} at node {id:?}")]
    IllegalTemporalNesting {
        /// The ID of the node where the violation occurred.
        id: ExprId,
        /// The kind of the parent temporal operator.
        parent_kind: ExprKind,
        /// The kind of the nested temporal operator that is forbidden.
        nested_kind: ExprKind,
    },

    /// A required sub-expression variant was missing from the scratchpad structural memoization cache.
    #[error("Cache Miss: A transformed child expression was expected but missing from the local scratchpad cache.")]
    CacheMiss,

    /// A structural logic error occurred where the root node failed to be reconstructed by the NNF loop.
    #[error("NNF Logic Error: The DFS transformation loop finished but the root expression was not successfully reconstructed.")]
    NnfLogicError,
}

impl ExprOpError {
    /// Creates an `IllegalTemporalNesting` error variant and captures the call site.
    #[track_caller]
    pub fn illegal_temporal_nesting(
        id: ExprId,
        parent_kind: ExprKind,
        nested_kind: ExprKind,
    ) -> Self {
        ExprOpError::IllegalTemporalNesting {
            id,
            parent_kind,
            nested_kind,
        }
        .trace()
    }

    /// Creates a `CacheMiss` error variant and captures the call site.
    #[track_caller]
    pub fn cache_miss() -> Self {
        ExprOpError::CacheMiss.trace()
    }

    /// Creates a `NnfLogicError` error variant and captures the call site.
    #[track_caller]
    pub fn nnf_logic_error() -> Self {
        ExprOpError::NnfLogicError.trace()
    }
}

impl Traceable for ExprOpError {}
