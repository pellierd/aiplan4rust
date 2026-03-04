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
/// the parsing and conversion of syntax tree nodes into expr.
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
}

impl ExprError {
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
                "Error captured at {file}:{line}:{col}\n\
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
        ExprError::InvalidAstNode { kind }.capture()
    }

    /// Creates an `ArithmeticEvaluationError` variant for a failed arithmetic operation and captures the call site.
    #[track_caller]
    pub fn arithmetic_evaluation_error(
        op: ArithmeticOp,
        values: Vec<OrderedFloat<f64>>,
    ) -> Self {
        ExprError::ArithmeticEvaluationError { op, values }.capture()
    }

    /// Creates an `InvalidExprNode` error variant for a node with an invalid kind and captures the call site.
    #[track_caller]
    pub fn invalid_expr_node(node_id: NodeId, kind: ExprKind) -> Self {
        ExprError::InvalidExprNode { node_id, kind }.capture()
    }

    /// Creates a `MissingTimeSpecifier` error variant for a literal node and captures the call site.
    #[track_caller]
    pub fn missing_time_specifier(node_id: NodeId) -> Self {
        ExprError::MissingTimeSpecifier { node_id }.capture()
    }

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    /// Captures the call site for easier debugging of translation failures.
    #[track_caller]
    pub fn unsupported_content(content: AstContent) -> Self {
        ExprError::UnsupportedContent { content }.capture()
    }

    /// Constructs a `NotQuantifierVariables` error and captures the call site.
    #[track_caller]
    pub fn not_quantifier_variables() -> Self {
        ExprError::NotQuantifierVariables.capture()
    }

    /// Constructs a `NotAtomSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_atom_skeleton() -> Self {
        ExprError::NotAtomSkeleton.capture()
    }

    /// Constructs a `NotConstant` error and captures the call site.
    #[track_caller]
    pub fn not_constant() -> Self {
        ExprError::NotConstant.capture()
    }

    /// Constructs a `NotVariable` error and captures the call site.
    #[track_caller]
    pub fn not_variable() -> Self {
        ExprError::NotVariable.capture()
    }

    /// Constructs a `NotFunctionSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_function_skeleton() -> Self {
        ExprError::NotFunctionSkeleton.capture()
    }

    /// Constructs a `NotPredicate` error and captures the call site.
    #[track_caller]
    pub fn not_predicate() -> Self {
        ExprError::NotPredicate.capture()
    }

    /// Constructs a `NotFunctor` error and captures the call site.
    #[track_caller]
    pub fn not_functor() -> Self {
        ExprError::NotFunctor.capture()
    }

    /// Constructs a `NotTaskSymbol` error and captures the call site.
    #[track_caller]
    pub fn not_task_symbol() -> Self {
        ExprError::NotTaskSymbol.capture()
    }

    /// Constructs a `NotTaskId` error and captures the call site.
    #[track_caller]
    pub fn not_task_id() -> Self {
        ExprError::NotTaskId.capture()
    }

    /// Constructs a `NotTaskSkeleton` error and captures the call site.
    #[track_caller]
    pub fn not_task_skeleton() -> Self {
        ExprError::NotTaskSkeleton.capture()
    }

    /// Constructs a `NotPreference` error and captures the call site.
    #[track_caller]
    pub fn not_preference() -> Self {
        ExprError::NotPreference.capture()
    }

    /// Constructs a `NotFloat` error and captures the call site.
    #[track_caller]
    pub fn not_float() -> Self {
        ExprError::NotFloat.capture()
    }

    /// Constructs a `NotBinaryComp` error and captures the call site.
    #[track_caller]
    pub fn not_binary_comp() -> Self {
        ExprError::NotBinaryComp.capture()
    }

    /// Constructs a `NotAssignOp` error and captures the call site.
    #[track_caller]
    pub fn not_assign_op() -> Self {
        ExprError::NotAssignOp.capture()
    }

    /// Constructs a `NotArithmeticOp` error and captures the call site.
    #[track_caller]
    pub fn not_arithmetic_op() -> Self {
        ExprError::NotArithmeticOp.capture()
    }

    /// Constructs a `NotOptimization` error and captures the call site.
    #[track_caller]
    pub fn not_optimization() -> Self {
        ExprError::NotOptimization.capture()
    }
}
