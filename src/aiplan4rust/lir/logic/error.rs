use ordered_float::OrderedFloat;
use thiserror::Error;
use crate::aiplan4rust::lang::{ArithmeticOp, LangError};
use crate::aiplan4rust::lir::analysis::inertia::InertiaError;
use crate::aiplan4rust::lir::expr::{ExprError, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;

#[derive(Error, Debug)]
pub enum LogicError {

    #[error(transparent)]
    Inertia(#[from] InertiaError),

    /// An error originating from the expr.
    #[error(transparent)]
    Expr(#[from] ExprError),

    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from the lang module
    #[error(transparent)]
    Lang(#[from] LangError),

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
    /// The node may be misplaced or of a type that cannot be processed in this context.
    #[error("Invalid expression node kind {kind:?} at node {node_id}")]
    InvalidExprNode {
        /// The ID of the expression node that is invalid.
        node_id: NodeId,
        /// The kind of the expression node that is invalid.
        kind: ExprKind,
    },

    /// Indicates that a literal node is not properly wrapped in a temporal specifier
    /// (`AtStart`, `AtEnd`, or `Overall`).
    #[error("Literal at node {node_id} is missing a temporal specifier")]
    MissingTimeSpecifier {
        /// The node ID of the literal missing a temporal specifier.
        node_id: NodeId,
    },

}

impl LogicError {
    /// Captures the current call site and backtrace for debugging purposes.
    ///
    /// This function logs the error, the location where capture() was called,
    /// and a full backtrace if the log level is set to Debug.
    #[track_caller]
    pub fn capture(self) -> Self {
        if log::log_enabled!(log::Level::Debug) {
            let caller = std::panic::Location::caller();
            let bt = std::backtrace::Backtrace::force_capture();

            log::debug!(
                "LogicError captured at {file}:{line}:{col}\n\
                 [Error] {error:?}\n\
                 [Stack Trace]\n{trace}",
                file = caller.file(),
                line = caller.line(),
                col = caller.column(),
                error = self,
                trace = bt
            );
        }
        self
    }

    /// Creates an `InvalidAstNode` error variant for a given `AstKind` and captures the call site.
    #[track_caller]
    pub fn invalid_ast_node(kind: AstKind) -> Self {
        LogicError::InvalidAstNode { kind }.capture()
    }

    /// Creates an `ArithmeticEvaluationError` variant for a failed arithmetic operation and captures the call site.
    #[track_caller]
    pub fn arithmetic_evaluation_error(
        op: ArithmeticOp,
        values: Vec<OrderedFloat<f64>>,
    ) -> Self {
        LogicError::ArithmeticEvaluationError { op, values }.capture()
    }

    /// Creates an `InvalidExprNode` error variant for a node with an invalid kind and captures the call site.
    #[track_caller]
    pub fn invalid_expr_node(node_id: NodeId, kind: ExprKind) -> Self {
        LogicError::InvalidExprNode { node_id, kind }.capture()
    }

    /// Creates a `MissingTimeSpecifier` error variant for a literal node and captures the call site.
    #[track_caller]
    pub fn missing_time_specifier(node_id: NodeId) -> Self {
        LogicError::MissingTimeSpecifier { node_id }.capture()
    }

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    /// Captures the call site for easier debugging of translation failures.
    #[track_caller]
    pub fn unsupported_content(content: AstContent) -> Self {
        LogicError::UnsupportedContent { content }.capture()
    }

}
