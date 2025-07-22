//! Error types for AST operations in `aiplan4rust`.
//!
//! This module defines [`AstError`], an enum representing possible errors
//! when working with the abstract syntax tree (AST). It reuses [`ArenaError`]
//! for arena-related errors and adds AST-specific failure modes such as
//! interner issues, span initialization errors, and serialization errors.

use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;

/// Errors that can occur when working with the AST.
#[derive(Error, Debug)]
pub enum AstError {
    /// An error occurred at the arena level.
    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    /// Catch-all internal error for unexpected conditions.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl AstError {

    pub fn internal_error(msg: impl Into<String>) -> Self {
        AstError::InternalError(msg.into())
    }
}
