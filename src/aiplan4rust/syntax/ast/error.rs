//! Error types for AST operations in `aiplan4rust`.
//!
//! This module defines [`AstError`], an enum representing possible errors
//! when working with the abstract syntax tree (AST). It reuses [`ArenaError`]
//! for arena-related errors and adds AST-specific failure modes such as
//! interner issues, span initialization errors, and serialization errors.

use thiserror::Error;
use std::io;
use crate::aiplan4rust::core::arena::ArenaError;
use serde_json;

/// Errors that can occur when working with the AST.
#[derive(Error, Debug)]
pub enum AstError {
    /// An error occurred at the arena level.
    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    /// The requested AST node was not found.
    #[error("AST node with id {0} not found")]
    NodeNotFound(usize),

    /// Error related to the string interner.
    #[error("Interner error: {0}")]
    InternerError(String),

    /// Error during span initialization.
    #[error("Span initialization error: {0}")]
    SpanInitializationError(String),

    /// IO error during serialization or deserialization.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Serialization or deserialization error (e.g. JSON).
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Generic traversal error.
    #[error("Traversal error: {0}")]
    TraversalError(String),

    /// Catch-all internal error for unexpected conditions.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl AstError {
    pub fn node_not_found(id: usize) -> Self {
        AstError::NodeNotFound(id)
    }

    pub fn interner_error(msg: impl Into<String>) -> Self {
        AstError::InternerError(msg.into())
    }

    pub fn span_initialization_error(msg: impl Into<String>) -> Self {
        AstError::SpanInitializationError(msg.into())
    }

    pub fn traversal_error(msg: impl Into<String>) -> Self {
        AstError::TraversalError(msg.into())
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        AstError::InternalError(msg.into())
    }
}
