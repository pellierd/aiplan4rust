//! This module defines the [`SyntaxError`] enum, representing errors
//! that can occur during parsing and syntax analysis within the system.
//!
//! It encapsulates errors from multiple subsystems such as the parser,
//! lexical analysis, AST construction, syntax tree handling, arena management,
//! parsing context validation, and source handling.
//!
//! # Overview of [`SyntaxError`] variants:
//! - [`ParseError`]: Errors originating from the LALRPOP parser, including lexical issues.
//! - [`Ast`]: Errors related to AST construction or manipulation.
//! - [`ParseContext`]: Errors in parsing context validation.
//! - [`SyntaxTee`]: Errors from syntax tree operations.
//! - [`Arena`]: Errors from the arena data structure managing nodes.
//! - [`Source`]: Errors related to source handling, including invalid, unknown, or serialized sources.
//!
//! The module also provides helper methods to extract the underlying parse error
//! and to create errors from source-related issues.

use thiserror::Error;
use lalrpop_util::ParseError;

use crate::aiplan4rust::syntax::lexer::Token;
use crate::aiplan4rust::syntax::CustomParseError;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::context::ParseContextError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Enum representing all possible syntax-related errors encountered
/// during parsing, AST processing, syntax tree handling, and source management.
///
/// This enum aggregates errors from various subsystems and wraps
/// them with meaningful messages for easier error management.
#[derive(Debug, Error)]
pub enum SyntaxError {
    /// Error returned by the parser during parsing.
    #[error(transparent)]
    ParseError(#[from] ParseError<usize, Token, CustomParseError>),

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

    /// Error originating from input source handling.
    ///
    /// This error is raised when loading or interpreting an [`Input`] fails,
    /// including:
    /// - I/O failures while reading a source file
    /// - Invalid or corrupted serialized IR headers
    /// - Unsupported or inconsistent input formats (e.g. mixed raw and IR sources)
    /// - Unknown or unrecognized source content
    ///
    /// This variant transparently wraps [`ArtefactError`] so that low-level I/O and
    /// deserialization details are preserved.
    #[error(transparent)]
    IO(#[from] ArtefactError),
}

impl SyntaxError {
    /// Returns the underlying parse error if this error is a `ParseError` variant.
    ///
    /// # Returns
    /// - `Some(&ParseError)` if this is a `ParseError`.
    /// - `None` otherwise.
    pub fn as_parse_error(&self) -> Option<&ParseError<usize, Token, CustomParseError>> {
        match self {
            SyntaxError::ParseError(ref err) => Some(err),
            _ => None,
        }
    }
}
