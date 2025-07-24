//! Error types for AST operations in `aiplan4rust`.
//!
//! This module defines [`AstError`], an enum representing possible errors
//! that may occur while working with the abstract syntax tree (AST).
//! It reuses [`SyntaxTreeError`] for syntax tree-specific issues and
//! introduces AST-specific errors such as unexpected content or internal logic faults.

use thiserror::Error;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Errors that can arise during AST manipulation.
#[derive(Error, Debug)]
pub enum AstError {

    /// Wraps errors originating from the syntax tree subsystem.
    #[error("Syntax tree error: {0}")]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Indicates that a required AST node was expected to be a `Requirement` but was not.
    #[error("Expected Requirement, but content was not a requirement")]
    NotARequirement,

    /// A general internal error for unexpected or invalid states.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl AstError {

    /// Creates a new `AstError::NotARequirement`.
    pub fn not_a_requirement() -> Self {
        AstError::NotARequirement
    }

    /// Creates a new `AstError::InternalError` with a given message.
    ///
    /// Useful for signaling unexpected logic errors or invariant violations.
    pub fn internal_error(msg: impl Into<String>) -> Self {
        AstError::InternalError(msg.into())
    }
}
