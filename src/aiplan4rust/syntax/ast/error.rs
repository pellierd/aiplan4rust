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

}

impl AstError {

    /// Creates a new `AstError::NotARequirement`.
    pub fn not_a_requirement() -> Self {
        AstError::NotARequirement
    }

}
