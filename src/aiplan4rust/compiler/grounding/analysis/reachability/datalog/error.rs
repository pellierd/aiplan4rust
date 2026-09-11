//! Datalog Error Management Module.
//!
//! This module defines the `DatalogError` enumeration, covering all structural,
//! expressiveness, and capacity constraints encountered during the grounding and
//! flattening phases of the Datalog pipeline. It integrates tightly with the
//! tracing infrastructure via the `Traceable` trait.

use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::support::lang::VariableId;
use thiserror::Error;

/// Errors encountered during the Datalog grounding and flattening process.
///
/// This enum covers inconsistencies in the internal state of the Datalog engine,
/// unsupported PDDL/LIR constructs, and capacity limits.
#[derive(Error, Debug)]
pub enum DatalogError {
    /// The construct is valid PDDL/ADL, but the current Datalog encoder
    /// has not implemented it yet.
    ///
    /// This typically applies to complex negations, implications, or quantifiers
    /// that require an explicit preprocessing step (like Positive Normal Form
    /// or Quantifier Expansion).
    #[error(
        "Feature not supported: {feature} at node {node_id:?}. \
     This ADL construct requires additional preprocessing or a more advanced encoder."
    )]
    FeatureNotSupported {
        /// Description of the unsupported feature.
        feature: String,
        /// The expression node identifier where the unsupported feature was found.
        node_id: ExprId,
    },

    /// The node encountered is fundamentally incompatible with Datalog grounding.
    ///
    /// This happens when the expression tree contains elements that cannot be
    /// mapped to Horn logic, such as HTN tasks, preferences, or internal
    /// compiler artifacts.
    #[error(
        "Incompatible node: {kind:?} at node {node_id:?}. \
     This element cannot be grounded into Datalog rules."
    )]
    IncompatibleExpr {
        /// The kind of the incompatible expression node.
        kind: ExprKind,
        /// The expression node identifier.
        node_id: ExprId,
    },

    /// Raised when a variable in the rule head is not bound by any atom in the body.
    #[error("Unbound variable '{0:?}' in rule head. All variables in the head must appear in the positive body.")]
    UnboundVariable(VariableId),

    /// Raised if an atom argument is neither a variable nor a constant.
    #[error("Invalid atom argument at node index {0}")]
    InvalidAtomArgument(ExprId),

    /// Raised when the variable index exceeds the maximum allowed limit per scope.
    #[error("Variable limit exceeded: Variable index '{var_id:?}' exceeds the maximum allowed variables per scope ({limit}).")]
    VariableLimitExceeded {
        /// The variable identifier that breached the limit.
        var_id: VariableId,
        /// The maximum allowed variables per scope.
        limit: usize,
    },

    /// Raised when a required internal segment (e.g., Type registry or Root node)
    /// has not been initialized before use.
    #[error("Internal engine state inconsistency: {0}")]
    InternalState(String),

    /// Errors propagated from the inertia table analysis.
    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    /// Errors propagated from the expression storer.
    #[error(transparent)]
    Store(#[from] StorerError),
}

impl Traceable for DatalogError {}

impl DatalogError {
    /// Creates a `FeatureNotSupported` error for missing ADL transformations.
    ///
    /// # Arguments
    ///
    /// * `feature` - A description or name of the unsupported feature.
    /// * `node_id` - The expression identifier where the feature was encountered.
    #[track_caller]
    pub fn feature_not_supported<S: Into<String>>(feature: S, node_id: ExprId) -> Self {
        DatalogError::FeatureNotSupported {
            feature: feature.into(),
            node_id,
        }
        .trace()
    }

    /// Creates an `IncompatibleExpr` error for nodes that don't belong in a Datalog pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of expression node.
    /// * `node_id` - The expression identifier that is incompatible.
    #[track_caller]
    pub fn incompatible_expr(kind: ExprKind, node_id: ExprId) -> Self {
        DatalogError::IncompatibleExpr { kind, node_id }.trace()
    }

    /// Creates an `InternalState` error with a custom message and captures the trace.
    ///
    /// # Arguments
    ///
    /// * `msg` - A descriptive message explaining the internal inconsistency.
    #[track_caller]
    pub fn internal_state<S: Into<String>>(msg: S) -> Self {
        DatalogError::InternalState(msg.into()).trace()
    }

    /// Creates an `InvalidAtomArgument` error for the specified node and captures the trace.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The expression node identifier with the invalid argument.
    #[track_caller]
    pub fn invalid_atom_argument(node_id: ExprId) -> Self {
        DatalogError::InvalidAtomArgument(node_id).trace()
    }

    /// Creates an `UnboundVariable` error for variables missing from the positive body.
    ///
    /// # Arguments
    ///
    /// * `v` - The unbound variable identifier.
    #[track_caller]
    pub fn unbound_variable(v: VariableId) -> Self {
        DatalogError::UnboundVariable(v).trace()
    }

    /// Creates a `VariableLimitExceeded` error when scope capacity is breached.
    ///
    /// # Arguments
    ///
    /// * `var_id` - The variable identifier exceeding limits.
    /// * `limit` - The maximum permissible threshold.
    #[track_caller]
    pub fn variable_limit_exceeded(var_id: VariableId, limit: usize) -> Self {
        DatalogError::VariableLimitExceeded { var_id, limit }.trace()
    }
}
