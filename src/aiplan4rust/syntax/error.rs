//! This module defines the [`SyntaxError`] enum, representing errors
//! that can occur during parsing and syntax analysis within the system.
//!
//! It encapsulates errors from multiple subsystems such as the parser,
//! lexical analysis, AST construction, syntax tree handling, arena management,
//! and parsing context validation.
//!
//! # Overview of [`SyntaxError`] variants:
//! - [`ParseError`]: Errors originating from the LALRPOP parser.
//! - [`Ast`]: Errors related to AST construction or manipulation.
//! - [`ParseContext`]: Errors in parsing context validation.
//! - [`SyntaxTee`]: Errors from syntax tree operations.
//! - [`Arena`]: Errors from the arena data structure.
//!
//! The module also provides helper methods to extract the underlying parse error
//! and create internal errors with custom messages.

use thiserror::Error;
use lalrpop_util::ParseError;
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::context::ParseContextError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Enum representing all possible syntax-related errors encountered
/// during parsing, AST processing, and syntax tree handling.
///
/// This enum aggregates errors from various subsystems and wraps
/// them with meaningful messages for easier error management.
///
/// # Variants
///
/// - `ParseError`: Errors during parsing, including lexical errors.
/// - `Ast`: Errors from AST manipulation.
/// - `ParseContext`: Errors related to parse context validation.
/// - `SyntaxTee`: Errors originating from syntax tree processing.
/// - `Arena`: Errors from the arena structure managing nodes.
/// - `InternalError`: General internal errors with a custom message.
#[derive(Debug, Error)]
pub enum SyntaxError {
    /// Error returned by the parser during parsing.
    #[error(transparent)]
    ParseError(#[from] ParseError<usize, Token, LexicalError>),

    /// Error related to Abstract Syntax Tree (AST) processing.
    #[error(transparent)]
    Ast(#[from] AstError),

    /// Error related to parsing context validation.
    #[error(transparent)]
    ParseContext(#[from] ParseContextError),

    /// Error from syntax tree operations.
    #[error(transparent)]
    SyntaxTee(#[from] SyntaxTreeError),

    /// Error from the arena node management subsystem.
    #[error(transparent)]
    Arena(#[from] ArenaError),
}

impl SyntaxError {
    /// Returns the underlying parse error if this error is a `ParseError` variant.
    ///
    /// # Returns
    /// - `Some(&ParseError)` if this is a `ParseError`.
    /// - `None` otherwise.
    pub fn as_parse_error(&self) -> Option<&ParseError<usize, Token, LexicalError>> {
        match self {
            SyntaxError::ParseError(ref err) => Some(err),
            _ => None,
        }
    }

}
