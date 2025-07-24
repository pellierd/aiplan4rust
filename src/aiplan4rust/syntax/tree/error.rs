use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;

/// Errors that can occur during syntax tree processing.
#[derive(Error, Debug)]
pub enum SyntaxTreeError {

    /// Wraps errors originating from arena allocation or manipulation.
    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    /// Indicates that the expected content was an identifier, but it was not.
    #[error("Expected Ident, but content was not an Ident")]
    NotAnIdent,

    /// Indicates that the expected content was a float, but it was not.
    #[error("Expected Float, but content was not a Float")]
    NotAFloat,

    /// Indicates that the expected content was a binary comparison operator, but it was not.
    #[error("Expected Binary comparison operator, but content was not one")]
    NotABinaryComp,

    /// Indicates that the expected content was an assignment operator, but it was not.
    #[error("Expected Assignment operator, but content was not one")]
    NotAnAssignOp,

    /// Indicates that the expected content was an arithmetic operator, but it was not.
    #[error("Expected Arithmetic operator, but content was not one")]
    NotAnArithmeticOp,

    /// Indicates that the expected content was an optimization directive, but it was not.
    #[error("Expected Optimization directive, but content was not one")]
    NotAnOptimization,

    /// Indicates that the expected content was a symbol reference, but it was not.
    #[error("Expected SymbolRef, but content was not a SymbolRef")]
    NotASymbolRef,

    /// A general internal error for unexpected conditions.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl SyntaxTreeError {
    /// Creates a new internal error with a custom message.
    pub fn internal_error(msg: impl Into<String>) -> Self {
        SyntaxTreeError::InternalError(msg.into())
    }

    /// Creates an error indicating content was not an identifier as expected.
    pub fn not_an_ident() -> Self {
        SyntaxTreeError::NotAnIdent
    }

    /// Creates an error indicating content was not a float as expected.
    pub fn not_a_float() -> Self {
        SyntaxTreeError::NotAFloat
    }

    /// Creates an error indicating content was not a binary comparison operator as expected.
    pub fn not_a_binary_comp() -> Self {
        SyntaxTreeError::NotABinaryComp
    }

    /// Creates an error indicating content was not an assignment operator as expected.
    pub fn not_an_assign_op() -> Self {
        SyntaxTreeError::NotAnAssignOp
    }

    /// Creates an error indicating content was not an arithmetic operator as expected.
    pub fn not_an_arithmetic_op() -> Self {
        SyntaxTreeError::NotAnArithmeticOp
    }

    /// Creates an error indicating content was not an optimization directive as expected.
    pub fn not_an_optimization() -> Self {
        SyntaxTreeError::NotAnOptimization
    }

    /// Creates an error indicating content was not a symbol reference as expected.
    pub fn not_a_symbol_ref() -> Self {
        SyntaxTreeError::NotASymbolRef
    }
}
