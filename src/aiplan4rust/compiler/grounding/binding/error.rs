use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluatorError;
use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// ### Binding Error
///
/// Represents the comprehensive set of errors that can occur during the variable
/// substitution (`binding`) and static evaluation phases of an expression tree.
///
/// This enum leverages the `thiserror` crate to provide transparent error propagation
/// and specialized diagnostic categorization across different compiler layers.
#[derive(Error, Debug)]
pub enum BindingError {
    /// Transparent forwarding of logical construction errors caught by the `ExprBuilder`
    /// (e.g., structural arity mismatches, invalid temporal scoping).
    #[error(transparent)]
    ExprBuilder(#[from] ExprBuilderError),

    /// Encapsulates dynamic evaluation errors triggered by an active static evaluator
    /// instance (such as Köhler pruning failures or state fluent evaluation faults).
    ///
    /// This variant uses a heap-allocated trait object (`Box<dyn ...>`) to prevent
    /// enum size inflation while supporting polymorphic error structures.
    #[error("Evaluation error: {0}")]
    Evaluator(Box<dyn ExprEvaluatorError>),
}

impl BindingError {}

impl Traceable for BindingError {}
