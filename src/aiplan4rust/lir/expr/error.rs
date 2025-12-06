use ordered_float::OrderedFloat;
use thiserror::Error;
use crate::aiplan4rust::lang::ArithmeticOp;
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

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

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    #[error("Unsupported content: {content:?}")]
    UnsupportedContent {
        /// The unsupported AST content variant that triggered the error.
        content: AstContent,
    },

    /// Indicates that an unsupported or unexpected `AstKind` variant was encountered.
    #[error("Unsupported kind: {kind:?}")]
    UnsupportedKind {
        /// The unsupported AST kind variant that triggered the error.
        kind: AstKind,
    },

    #[error("Arithmetic  Evaluation error in operation {op:?} with operands {values:?}")]
    ArithmeticEvaluationError {
        op: ArithmeticOp,
        values: Vec<OrderedFloat<f64>>,
    },

    /// Erreur lorsqu’un nœud inattendu est rencontré dans un arbre normalisé
    #[error("Unexpected node kind {kind:?} at node {node_id}")]
    UnexpectedNodeKind {
        node_id: NodeId,
        kind: ExprKind,
    },

    /// Erreur lorsqu’un littéral n’est pas sous un temporal specifier
    #[error("Literal at node {node_id} is missing a temporal specifier")]
    MissingTimeSpecifier {
        /// The node ID of the literal missing a time specifier
        node_id: NodeId,
    },
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

    /// Creates an `UnsupportedKind` error variant for the given `AstKind`.
    ///
    /// # Arguments
    ///
    /// * `kind` - The unsupported AST kind variant encountered.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the unsupported kind error.
    pub fn unsupported_kind(kind: AstKind) -> Self {
        ExprError::UnsupportedKind { kind }
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

    /// Creates an `UnexpectedNodeKind` error variant for a node with an unexpected kind.
    ///
    /// # Arguments
    ///
    /// * `node_id` – The ID of the node that triggered the error.
    /// * `kind` – The kind of the node that was unexpected.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the unexpected node kind.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use your_crate::{ExprError, ExprKind, NodeId};
    /// let err = ExprError::unexpected_node_kind(42, ExprKind::And);
    /// ```
    pub fn unexpected_node_kind(node_id: NodeId, kind: ExprKind) -> Self {
        ExprError::UnexpectedNodeKind { node_id, kind }
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
}
