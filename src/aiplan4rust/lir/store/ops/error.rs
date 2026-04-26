use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::lang::{ArithmeticOp, LangError};
use crate::aiplan4rust::lir::expr::{ExprError, ExprKind};
use crate::aiplan4rust::lir::store::builder::ExprBuilderError;
use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use ordered_float::OrderedFloat;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExprOpErrorHC {
    #[error(transparent)]
    Store(#[from] StorerError),

    #[error(transparent)]
    Inertia(#[from] InertiaError),

    /// An error originating from the logic.
    #[error(transparent)]
    Expr(#[from] ExprError),

    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from the lang module
    #[error(transparent)]
    Lang(#[from] LangError),

    #[error(transparent)]
    ExpBuilder(#[from] ExprBuilderError),

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    #[error("Unsupported content: {content:?}")]
    UnsupportedContent {
        /// The unsupported AST content variant that triggered the error.
        content: AstContent,
    },

    /// Indicates that an AST node cannot be translated into the intermediate representation (IR)
    /// because its kind is unsupported by the pipeline.
    #[error("Unsupported AST kind: {kind:?}")]
    InvalidAstNode {
        /// The unsupported AST kind variant that triggered the error.
        kind: AstKind,
    },

    /// Indicates an error during arithmetic evaluation of an operation with given operands.
    #[error("Arithmetic evaluation error in operation {op:?} with operands {values:?}")]
    ArithmeticEvaluationError {
        /// The arithmetic operation that failed.
        op: ArithmeticOp,
        /// The operand values that caused the error.
        values: Vec<OrderedFloat<f64>>,
    },

    /// Indicates that an expression node in the IR is invalid for the current transformation.
    /// The node may be misplaced or of a typing that cannot be processed in this context.
    #[error("Invalid expression node kind {kind:?} at node {node_id}")]
    InvalidExprNode {
        /// The ID of the expression node that is invalid.
        node_id: NodeId,
        /// The kind of the expression node that is invalid.
        kind: ExprKind,
    },

    /// Indicates that a literal node is not properly wrapped in a temporal specifier
    /// (`AtStart`, `AtEnd`, or `Overall`).
    #[error("Temporal Consistency Error: Literal at node {id} ({kind:?}) is missing a temporal specifier. All atomic formulas must be wrapped in 'at start', 'at end', or 'overall'.")]
    MissingTimeSpecifier {
        /// The ID of the expression node (ExprId) missing a temporal specifier.
        id: ExprId,
        /// The kind of the node to help debugging.
        kind: ExprEntryKind,
    },

    /// Indicates an illegal nesting of temporal operators (e.g., 'at start' inside 'at end').
    /// This is typically caught during parsing or initial expression building.
    #[error("Illegal Temporal Nesting: Cannot nest temporal operator {nested_kind:?} inside {parent_kind:?} at node {id:?}")]
    IllegalTemporalNesting {
        /// The ID of the node where the violation occurred.
        id: ExprId,
        /// The kind of the parent temporal operator.
        parent_kind: ExprEntryKind,
        /// The kind of the nested temporal operator that is forbidden.
        nested_kind: ExprEntryKind,
    },
}

impl ExprOpErrorHC {
    /// Creates an `InvalidAstNode` error variant for a given `AstKind` and captures the call site.
    #[track_caller]
    pub fn invalid_ast_node(kind: AstKind) -> Self {
        ExprOpErrorHC::InvalidAstNode { kind }.trace()
    }

    /// Creates an `ArithmeticEvaluationError` variant for a failed arithmetic operation and captures the call site.
    #[track_caller]
    pub fn arithmetic_evaluation_error(op: ArithmeticOp, values: Vec<OrderedFloat<f64>>) -> Self {
        ExprOpErrorHC::ArithmeticEvaluationError { op, values }.trace()
    }

    /// Creates an `InvalidExprNode` error variant for a node with an invalid kind and captures the call site.
    #[track_caller]
    pub fn invalid_expr_node(node_id: NodeId, kind: ExprKind) -> Self {
        ExprOpErrorHC::InvalidExprNode { node_id, kind }.trace()
    }

    #[track_caller]
    pub fn missing_time_specifier(id: ExprId, kind: ExprEntryKind) -> Self {
        ExprOpErrorHC::MissingTimeSpecifier { id, kind }.trace()
    }

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    /// Captures the call site for easier debugging of translation failures.
    #[track_caller]
    pub fn unsupported_content(content: AstContent) -> Self {
        ExprOpErrorHC::UnsupportedContent { content }.trace()
    }

    /// Creates an `IllegalTemporalNesting` error variant and captures the call site.
    #[track_caller]
    pub fn illegal_temporal_nesting(
        id: ExprId,
        parent_kind: ExprEntryKind,
        nested_kind: ExprEntryKind,
    ) -> Self {
        ExprOpErrorHC::IllegalTemporalNesting {
            id,
            parent_kind,
            nested_kind,
        }
        .trace()
    }
}

impl Traceable for ExprOpErrorHC {}
