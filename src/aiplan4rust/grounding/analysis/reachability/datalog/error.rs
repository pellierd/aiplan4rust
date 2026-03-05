use thiserror::Error;
use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::expr::{ExprError, ExprKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::error::Traceable;

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
    /// that require an explicit Preprocessing step (like Positive Normal Form
    /// or Quantifier Expansion).
    #[error("Feature not supported: {feature} at node {node_id:?}. \
             This ADL construct requires additional preprocessing or a more advanced encoder.")]
    FeatureNotSupported {
        feature: String,
        node_id: NodeId,
    },

    /// The node encountered is fundamentally incompatible with Datalog grounding.
    ///
    /// This happens when the expression tree contains elements that cannot be
    /// mapped to Horn logic, such as HTN tasks, preferences, or internal
    /// compiler artifacts.
    #[error("Incompatible node: {kind:?} at node {node_id:?}. \
             This element cannot be grounded into Datalog rules.")]
    IncompatibleNode {
        kind: ExprKind,
        node_id: NodeId,
    },

    /// Raised if an atom argument is neither a variable nor a constant.
    #[error("Invalid atom argument at node index {0}")]
    InvalidAtomArgument(NodeId),

    /// Raised when a required internal segment (e.g., Type registry or Root node)
    /// has not been initialized before use.
    #[error("Internal engine state inconsistency: {0}")]
    InternalState(String),

    /// Errors propagated from the underlying syntax tree manipulation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Errors propagated from the Low-level Intermediate Representation (LIR) layer.
    #[error(transparent)]
    Expr(#[from] ExprError),
}

impl Traceable for DatalogError {}

impl DatalogError {
    /// Creates a `FeatureNotSupported` error, typically for missing ADL transformations.
    #[track_caller]
    pub fn feature_not_supported<S: Into<String>>(feature: S, node_id: NodeId) -> Self {
        DatalogError::FeatureNotSupported {
            feature: feature.into(),
            node_id,
        }.trace()
    }

    /// Creates an `IncompatibleNode` error for nodes that don't belong in a Datalog pipeline.
    #[track_caller]
    pub fn incompatible_node(kind: ExprKind, node_id: NodeId) -> Self {
        DatalogError::IncompatibleNode { kind, node_id }.trace()
    }

    /// Creates an `InternalState` error with a custom message and captures the trace.
    #[track_caller]
    pub fn internal_state<S: Into<String>>(msg: S) -> Self {
        DatalogError::InternalState(msg.into()).trace()
    }

    /// Creates an `InvalidAtomArgument` error for the specified node and captures the trace.
    #[track_caller]
    pub fn invalid_atom_argument(node_id: NodeId) -> Self {
        DatalogError::InvalidAtomArgument(node_id).trace()
    }

}
