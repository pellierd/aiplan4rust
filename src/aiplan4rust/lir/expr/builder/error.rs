use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::expr::ExprEntryKind;
use thiserror::Error;

/// Errors encountered during the construction of an expression via [`ExprBuilder`].
///
/// This enum covers both logical violations (like PDDL semantic constraints)
/// and low-level storage failures.
#[derive(Error, Debug)]
pub enum ExprBuilderError {
    /// Occurs when a quantifier (Forall/Exists) or a function declaration
    /// attempts to reuse a [`VariableId`] that is already defined in the current scope.
    #[error("duplicate variable declaration detected: {0}")]
    DuplicateVariable(VariableId),

    /// Violation of PDDL temporal syntax: temporal operators cannot be nested.
    ///
    /// According to PDDL semantics, you cannot nest temporal operators such as
    /// `(at start (at start ...))`. This error is triggered when the builder
    /// detects an illegal nesting during construction.
    #[error(
        "PDDL Semantic Error: cannot wrap {attempted_kind:?} around an existing {existing_kind:?}"
    )]
    InvalidTemporalInvariant {
        /// The operator already present in the sub-expression.
        existing_kind: ExprEntryKind,
        /// The operator that was attempted to be wrapped around it.
        attempted_kind: ExprEntryKind,
    },

    /// Triggered when a timestamp is negative where a non-negative value is required.
    #[error("PDDL Semantic Error: timestamp must be non-negative, found {0}")]
    InvalidTimestamp(f64),

    /// Wraps errors originating from the underlying [`ExprStore`].
    ///
    /// This typically includes issues like reaching storage capacity limits
    /// or internal integrity violations.
    #[error("store error: {0}")]
    Store(#[from] StorerError),
}

impl ExprBuilderError {
    /// Creates a [`DuplicateVariable`](Self::DuplicateVariable) error and captures the call site.
    ///
    /// Uses `#[track_caller]` to ensure that the trace points to the code
    /// that called this constructor, rather than the constructor's body.
    #[track_caller]
    pub fn duplicate_variable(var: VariableId) -> Self {
        ExprBuilderError::DuplicateVariable(var).trace()
    }

    /// Creates an [`InvalidTemporalInvariant`](Self::InvalidTemporalInvariant) error and captures the call site.
    ///
    /// # Arguments
    /// * `existing_kind` - The kind of the inner expression node.
    /// * `attempted_kind` - The kind of the temporal operator we tried to apply.
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

    #[track_caller]
    pub fn invalid_timestamp(time: f64) -> Self {
        ExprBuilderError::InvalidTimestamp(time).trace()
    }
}

/// Allows [`ExprBuilderError`] to be augmented with stack trace information.
impl Traceable for ExprBuilderError {}
