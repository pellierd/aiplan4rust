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

    /// Raised when an unsupported or unexpected node kind is encountered during flattening.
    ///
    /// **Note:** This often indicates that the quantifier expansion or ADL preprocessing
    /// steps were skipped or failed to simplify the expression.
    #[error("Unsupported node type {kind:?} at node {node_id:?}. Ensure expand() was called.")]
    UnsupportedNode {
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
    /// Creates an `UnsupportedNode` error and captures the call site trace.
    ///
    /// # Arguments
    /// * `kind` - The kind of expression that is not supported.
    /// * `node_id` - The ID of the node in the expression tree.
    #[track_caller]
    pub fn unsupported_node(kind: ExprKind, node_id: NodeId) -> Self {
        DatalogError::UnsupportedNode { kind, node_id }.trace()
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
