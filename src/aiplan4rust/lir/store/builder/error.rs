use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::ExprEntryKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExprBuilderError {
    #[error("duplicate variable declaration detected: {0}")]
    DuplicateVariable(VariableId),

    /// Violation of PDDL temporal syntax: temporal operators cannot be nested.
    #[error(
        "PDDL Semantic Error: cannot wrap {attempted_kind:?} around an existing {existing_kind:?}"
    )]
    InvalidTemporalInvariant {
        /// The operator already present in the sub-expression.
        existing_kind: ExprEntryKind,
        /// The operator that was attempted to be wrapped around it.
        attempted_kind: ExprEntryKind,
    },

    #[error("store error: {0}")]
    Store(#[from] StorerError),
}

impl ExprBuilderError {
    /// Creates a `DuplicateVariable` error and captures the call site.
    #[track_caller]
    pub fn duplicate_variable(var: VariableId) -> Self {
        ExprBuilderError::DuplicateVariable(var).trace()
    }

    /// Creates an `InvalidTemporalInvariant` error and captures the call site.
    #[track_caller]
    pub fn invalid_temporal_invariant(
        existing_kind: ExprEntryKind,
        attempted_kind: ExprEntryKind,
    ) -> Self {
        ExprBuilderError::InvalidTemporalInvariant {
            existing_kind,
            attempted_kind,
        }
        .trace()
    }
}

impl Traceable for ExprBuilderError {}
