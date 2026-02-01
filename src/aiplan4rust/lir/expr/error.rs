use ordered_float::OrderedFloat;
use thiserror::Error;
use crate::aiplan4rust::lang::{ArithmeticOp, LangError};
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;

/// Errors specific to the `expr` module, primarily related to conversion failures.
///
/// This enum represents the various error conditions that can occur during
/// the parsing and conversion of syntax tree nodes into expressions.
///
/// # Variants
///
/// - [`SyntaxTree`]: Wraps errors originating from the underlying syntax tree system.
/// - [`UnsupportedContent`]: Indicates that an unexpected or unsupported `AstContent` variant was encountered.
/// - [`UnsupportedKind`]: Indicates that an unexpected or unsupported `AstKind` variant was encountered.
/// - [`InternalError`]: Represents generic internal errors for unexpected states or conditions.
///
/// # Usage
///
/// These errors are typically returned during the transformation from
/// syntax tree representations to intermediate expression forms, allowing
/// detailed reporting of unsupported or invalid constructs.
///
/// # Examples
///
/// ```
/// use aiplan4rust::lir::expr::ExprError;
/// use aiplan4rust::syntax::ast::{AstContent, AstKind};
///
/// let err = ExprError::unsupported_content(AstContent::Ident(42));
/// let err_kind = ExprError::unsupported_kind(AstKind::EffectDef);
/// let internal = ExprError::internal_error("Unexpected null value");
/// ```
#[derive(Error, Debug)]
pub enum ExprError {
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

    /// Indicates that the expected content was a quantifier variables list, but it was not.
    #[error("Expected quantifier variables, but content was not QuantifierVariables")]
    NotQuantifierVariables,
}

impl ExprError {
    /// Creates an `UnsupportedContent` error variant for the given `AstContent`.
    ///
    /// # Arguments
    ///
    /// * `content` - The unsupported AST content variant encountered.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the unsupported content error.
    pub fn unsupported_content(content: AstContent) -> Self {
        ExprError::UnsupportedContent { content }
    }

    /// Creates an `InvalidAstNode` error variant for a given `AstKind`.
    ///
    /// This error indicates that the AST node cannot be translated into the
    /// intermediate representation (IR) because it is **unsupported** in the current pipeline.
    ///
    /// # Arguments
    ///
    /// * `kind` - The AST kind that is invalid or unsupported.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing that this AST node cannot be processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use your_crate::{ExprError, AstKind};
    /// let err = ExprError::invalid_ast_node(AstKind::Task);
    /// ```
    pub fn invalid_ast_node(kind: AstKind) -> Self {
        ExprError::InvalidAstNode { kind }
    }

    /// Creates an `ArithmeticEvaluationError` variant for a failed arithmetic operation.
    ///
    /// # Arguments
    ///
    /// * `op` – the arithmetic operation that caused the error.
    /// * `values` – the operands involved in the operation.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the arithmetic evaluation failure.
    pub fn arithmetic_evaluation_error(
        op: ArithmeticOp,
        values: Vec<OrderedFloat<f64>>,
    ) -> Self {
        ExprError::ArithmeticEvaluationError { op, values }
    }

    /// Creates an `InvalidExprNode` error variant for a node with an invalid kind
    /// in the context of the current transformation.
    ///
    /// This error indicates that the expression node exists in the intermediate
    /// representation (IR) but **cannot be processed** by the current operation
    /// because it is either misplaced or of a type that is not supported
    /// in this context.
    ///
    /// # Arguments
    ///
    /// * `node_id` – The ID of the node that triggered the error.
    /// * `kind` – The kind of the expression node that is invalid.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing that this expression node cannot be processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use your_crate::{ExprError, ExprKind, NodeId};
    /// let err = ExprError::invalid_expr_node(42, ExprKind::AtStart);
    /// ```
    pub fn invalid_expr_node(node_id: NodeId, kind: ExprKind) -> Self {
        ExprError::InvalidExprNode { node_id, kind }
    }

    /// Creates a `MissingTimeSpecifier` error variant for a literal node that is missing a temporal specifier.
    ///
    /// # Arguments
    ///
    /// * `node_id` – The ID of the literal node that is missing a temporal specifier.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the missing time specifier error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use your_crate::{ExprError, NodeId};
    /// let err = ExprError::missing_time_specifier(101);
    /// ```
    pub fn missing_time_specifier(node_id: NodeId) -> Self {
        ExprError::MissingTimeSpecifier { node_id }
    }

    /// Constructs a `NotQuantifierVariables` error.
    pub fn not_quantifier_variables() -> Self {
        ExprError::NotQuantifierVariables
    }

}
