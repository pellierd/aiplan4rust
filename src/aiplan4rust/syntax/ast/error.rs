//! Error types for AST operations in `aiplan4rust`.
//!
//! This module defines [`AstError`], an enum representing possible errors
//! when working with the abstract syntax tree (AST). It reuses [`ArenaError`]
//! for arena-related errors and adds AST-specific failure modes such as
//! interner issues, span initialization errors, and serialization errors.

use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Errors that can occur when working with the AST.
#[derive(Error, Debug)]
pub enum AstError {

    #[error("Syntax tree error: {0}")]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error("Expected Requirement, but content was not a requirement")]
    NotARequirement,

    /// Catch-all internal error for unexpected conditions.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl AstError {

    /// Constructs a new `AstError::NotARequirement`.
    pub fn not_a_requirement() -> Self {
        AstError::NotARequirement
    }
    pub fn internal_error(msg: impl Into<String>) -> Self {
        AstError::InternalError(msg.into())
    }

}
