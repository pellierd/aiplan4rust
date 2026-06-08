//! Error types for AST operations in `aiplan4rust`.
//!
//! This module defines [`AstError`], an enum representing possible errors
//! that may occur while working with the abstract syntax tree (AST).
//! It reuses [`SyntaxTreeError`] for syntax tree-specific issues and
//! introduces AST-specific errors such as unexpected content or internal ops faults.

use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

/// Errors that can arise during AST manipulation.
#[derive(Error, Debug)]
pub enum AstError {
    /// Wraps errors originating from the syntax tree subsystem.
    #[error("Syntax tree error: {0}")]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Indicates that a required AST node was expected to be a `Requirement` but was not.
    #[error("Expected Requirement, but content was not a requirement")]
    NotARequirement,

    #[error("Expected symbol id, but content was not a symbol id")]
    NotASymbolID,
}

impl AstError {
    /// Creates a new `AstError::NotARequirement`.
    pub fn not_a_requirement() -> Self {
        AstError::NotARequirement
    }

    pub fn not_a_symbol_id() -> Self {
        let caller = std::panic::Location::caller();
        let err = AstError::NotASymbolID;

        if log::log_enabled!(log::Level::Debug) {
            let bt = std::backtrace::Backtrace::force_capture();

            log::debug!(
                "\nAST Error at {}:{}:{}\n{}\nStack trace:\n{}",
                caller.file(),
                caller.line(),
                caller.column(),
                err,
                bt
            );
        }

        err
    }
}
