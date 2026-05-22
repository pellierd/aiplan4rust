use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::{ArithmeticOp, LangError};
use crate::aiplan4rust::lir::expr::ExprId;
use crate::aiplan4rust::lir::old::expr::ExprKind;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use ordered_float::OrderedFloat;
use thiserror::Error;

/// Errors specific to the `logic` module, primarily related to conversion failures.
///
/// This enum represents the various error conditions that can occur during
/// the parsing and conversion of syntax tree nodes into logic.
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
/// use aiplan4rust::lir::logic::ExprError;
/// use aiplan4rust::syntax::ast::{AstContent, AstKind};
///
/// let err = ExprError::unsupported_content(AstContent::Ident(42));
/// let err_kind = ExprError::unsupported_kind(AstKind::EffectDef);
/// let internal = ExprError::internal_error("Unexpected null value");
/// ```
#[derive(Error, Debug)]
pub enum StorerError {
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
    #[error("Literal at node {node_id} is missing a temporal specifier")]
    MissingTimeSpecifier {
        /// The node ID of the literal missing a temporal specifier.
        node_id: NodeId,
    },

    /// Indicates that the expected content was a quantifier variables list, but it was not.
    #[error("Expected quantifier variables, but content was not QuantifierVariables")]
    NotQuantifierVariables,

    /// Indicates that the expected content was an atomic skeleton (predicate), but it was not.
    #[error("Expected atomic skeleton, but content was not AtomicSkeleton")]
    NotAtomSkeleton,

    /// Indicates that the expected content was a constant (object), but it was not.
    #[error("Expected constant, but content was not Constant")]
    NotConstant,

    /// Indicates that the expected content was a variable, but it was not.
    #[error("Expected variable, but content was not Variable")]
    NotVariable,

    /// Indicates that the expected content was a function skeleton, but it was not.
    #[error("Expected function skeleton, but content was not FunctionSkeleton")]
    NotFunctionSkeleton,

    /// Indicates that the expected content was a predicate ID, but it was not.
    #[error("Expected predicate, but content was not Predicate")]
    NotPredicate,

    /// Indicates that the expected content was a functor, but it was not.
    #[error("Expected functor, but content was not Functor")]
    NotFunctor,

    /// Indicates that the expected content was a task symbol, but it was not.
    #[error("Expected task symbol, but content was not TaskSymbol")]
    NotTaskSymbol,

    /// Indicates that the expected content was a task label, but it was not.
    #[error("Expected task ID, but content was not TaskID")]
    NotTaskId,

    /// Indicates that the expected content was a task skeleton, but it was not.
    #[error("Expected task skeleton, but content was not TaskSkeleton")]
    NotTaskSkeleton,

    /// Indicates that the expected content was a preference, but it was not.
    #[error("Expected preference, but content was not Preference")]
    NotPreference,

    /// Indicates that the expected content was a float value, but it was not.
    #[error("Expected float, but content was not Float")]
    NotFloat,

    /// Indicates that the expected content was a binary comparison operator, but it was not.
    #[error("Expected binary comparison operator, but content was not BinaryComp")]
    NotBinaryComp,

    /// Indicates that the expected content was an assignment operator, but it was not.
    #[error("Expected assignment operator, but content was not AssignOp")]
    NotAssignOp,

    /// Indicates that the expected content was an arithmetic operator, but it was not.
    #[error("Expected arithmetic operator, but content was not ArithmeticOp")]
    NotArithmeticOp,

    /// Indicates that the expected content was an optimization directive, but it was not.
    #[error("Expected optimization directive, but content was not Optimization")]
    NotOptimization,

    /// Indicates that an expression ID does not exist in the old.
    #[error("Expression with ID {id} was not found in the old")]
    ExprNotFound {
        /// The index/ID that failed to be retrieved.
        id: ExprId,
    },
}

impl StorerError {
    /// Creates an `InvalidAstNode` error variant for a given `AstKind` and captures the call site.
    #[track_caller]
    pub fn invalid_ast_node(kind: AstKind) -> Self {
        StorerError::InvalidAstNode { kind }.trace()
    }

    /// Creates an `ArithmeticEvaluationError` variant for a failed arithmetic operation and captures the call site.
    #[track_caller]
    pub fn arithmetic_evaluation_error(op: ArithmeticOp, values: Vec<OrderedFloat<f64>>) -> Self {
        StorerError::ArithmeticEvaluationError { op, values }.trace()
    }

    /// Creates an `InvalidExprNode` error variant for a node with an invalid kind and captures the call site.
    #[track_caller]
    pub fn invalid_expr_node(node_id: NodeId, kind: ExprKind) -> Self {
        StorerError::InvalidExprNode { node_id, kind }.trace()
    }

    /// Creates a `MissingTimeSpecifier` error variant for a literal node and captures the call site.
    #[track_caller]
    pub fn missing_time_specifier(node_id: NodeId) -> Self {
        StorerError::MissingTimeSpecifier { node_id }.trace()
    }

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    /// Captures the call site for easier debugging of translation failures.
    #[track_caller]
    pub fn unsupported_content(content: AstContent) -> Self {
        StorerError::UnsupportedContent { content }.trace()
    }

    /// Constructs a `NotQuantifierVariables` error and captures the call site.
    #[track_caller]
    pub fn not_quantifier_variables() -> Self {
        StorerError::NotQuantifierVariables.trace()
    }

    /// Constructs a `NotAtomSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_atom_skeleton() -> Self {
        StorerError::NotAtomSkeleton.trace()
    }

    /// Constructs a `NotConstant` error and captures the call site.
    #[track_caller]
    pub fn not_constant() -> Self {
        StorerError::NotConstant.trace()
    }

    /// Constructs a `NotVariable` error and captures the call site.
    #[track_caller]
    pub fn not_variable() -> Self {
        StorerError::NotVariable.trace()
    }

    /// Constructs a `NotFunctionSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_function_skeleton() -> Self {
        StorerError::NotFunctionSkeleton.trace()
    }

    /// Constructs a `NotPredicate` error and captures the call site.
    #[track_caller]
    pub fn not_predicate() -> Self {
        StorerError::NotPredicate.trace()
    }

    /// Constructs a `NotFunctor` error and captures the call site.
    #[track_caller]
    pub fn not_functor() -> Self {
        StorerError::NotFunctor.trace()
    }

    /// Constructs a `NotTaskSymbol` error and captures the call site.
    #[track_caller]
    pub fn not_task_symbol() -> Self {
        StorerError::NotTaskSymbol.trace()
    }

    /// Constructs a `NotTaskId` error and captures the call site.
    #[track_caller]
    pub fn not_task_id() -> Self {
        StorerError::NotTaskId.trace()
    }

    /// Constructs a `NotTaskSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_task_skeleton() -> Self {
        StorerError::NotTaskSkeleton.trace()
    }

    /// Constructs a `NotPreference` error and captures the call site.
    #[track_caller]
    pub fn not_preference() -> Self {
        StorerError::NotPreference.trace()
    }

    /// Constructs a `NotFloat` error and captures the call site.
    #[track_caller]
    pub fn not_float() -> Self {
        StorerError::NotFloat.trace()
    }

    /// Constructs a `NotBinaryComp` error and captures the call site.
    #[track_caller]
    pub fn not_binary_comp() -> Self {
        StorerError::NotBinaryComp.trace()
    }

    /// Constructs a `NotAssignOp` error and captures the call site.
    #[track_caller]
    pub fn not_assign_op() -> Self {
        StorerError::NotAssignOp.trace()
    }

    /// Constructs a `NotArithmeticOp` error and captures the call site.
    #[track_caller]
    pub fn not_arithmetic_op() -> Self {
        StorerError::NotArithmeticOp.trace()
    }

    /// Constructs a `NotOptimization` error and captures the call site.
    #[track_caller]
    pub fn not_optimization() -> Self {
        StorerError::NotOptimization.trace()
    }

    /// Creates an `ExprNotFound` error and captures the call site.
    #[track_caller]
    pub fn expr_not_found(id: ExprId) -> Self {
        StorerError::ExprNotFound { id }.trace()
    }
}

impl Traceable for StorerError {}
