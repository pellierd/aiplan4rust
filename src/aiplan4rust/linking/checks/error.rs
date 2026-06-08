//! Defines the `LinkingCheckError` enum representing errors
//! that can occur specifically during the linking checks phase
//! in the AIPlan4Rust compilation pipeline.
//!
//! This enum consolidates errors originating from common subsystems
//! such as symbol table operations, syntax tree processing,
//! and string interning, all of which are essential during linking validation.
//!
//! # Variants
//!
//! - [`SymbolTable`]: Wraps errors from symbol table operations such as lookups or insertions.
//! - [`SyntaxTree`]: Wraps errors related to the construction or manipulation of the syntax tree (AST).
//! - [`Interner`]: Wraps errors originating from string interner operations, e.g., identifier resolution failures.
//!
//! # Usage
//!
//! Implements the `std::error::Error` trait via `thiserror::Error`
//! for seamless error composition and propagation.

use crate::aiplan4rust::core::interner::InternerError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

/// Represents errors that may occur during the linking checks phase.
///
/// This enum aggregates errors arising during various linking
/// validation steps, ensuring that symbol table, syntax tree,
/// and interner errors can be handled uniformly.
#[derive(Debug, Error)]
pub enum LinkingCheckError {
    /// Error from symbol table operations such as lookup or insertion.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Error related to syntax tree construction or manipulation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Error originating from string interner operations.
    #[error(transparent)]
    Interner(#[from] InternerError),
}
