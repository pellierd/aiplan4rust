//! Syntax Tree Error Module
//!
//! This module defines the `SyntaxTreeError` enum, which encapsulates
//! all errors that can occur during the processing and manipulation
//! of syntax trees within the system.
//!
//! These errors primarily arise from type mismatches when
//! extracting semantic content from syntax nodes, or from arena
//! allocation errors.
//!
//! The enum variants provide detailed and specific error types to
//! facilitate precise error handling and debugging during parsing,
//! syntax analysis, and transformation phases.
//!
//! # Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
//!
//! fn extract_ident(node: &SyntaxNode) -> Result<Ident, SyntaxTreeError> {
//!     node.try_ident().map_err(|_| SyntaxTreeError::not_an_ident())
//! }
//! ```

use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;

/// Errors that can occur during syntax tree processing.
///
/// This enum represents various failure cases encountered when
/// working with syntax trees, including semantic content extraction
/// mismatches and arena-related errors.
#[derive(Error, Debug)]
pub enum SyntaxTreeError {
    /// Wraps errors originating from arena allocation or manipulation.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Indicates that the expected content was an identifier, but it was not.
    #[error("Expected Ident, but content was not an Ident")]
    NotAnIdent,

    /// Indicates that the expected content was a floating-point number, but it was not.
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
}

impl SyntaxTreeError {
    /// Creates an error indicating that the content was not an identifier as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotAnIdent` error variant.
    pub fn not_an_ident() -> Self {
        SyntaxTreeError::NotAnIdent
    }

    /// Creates an error indicating that the content was not a float as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotAFloat` error variant.
    pub fn not_a_float() -> Self {
        SyntaxTreeError::NotAFloat
    }

    /// Creates an error indicating that the content was not a binary comparison operator as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotABinaryComp` error variant.
    pub fn not_a_binary_comp() -> Self {
        SyntaxTreeError::NotABinaryComp
    }

    /// Creates an error indicating that the content was not an assignment operator as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotAnAssignOp` error variant.
    pub fn not_an_assign_op() -> Self {
        SyntaxTreeError::NotAnAssignOp
    }

    /// Creates an error indicating that the content was not an arithmetic operator as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotAnArithmeticOp` error variant.
    pub fn not_an_arithmetic_op() -> Self {
        SyntaxTreeError::NotAnArithmeticOp
    }

    /// Creates an error indicating that the content was not an optimization directive as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotAnOptimization` error variant.
    pub fn not_an_optimization() -> Self {
        SyntaxTreeError::NotAnOptimization
    }

    /// Creates an error indicating that the content was not a symbol reference as expected.
    ///
    /// # Returns
    ///
    /// A `SyntaxTreeError::NotASymbolRef` error variant.
    pub fn not_a_symbol_ref() -> Self {
        SyntaxTreeError::NotASymbolRef
    }
}
