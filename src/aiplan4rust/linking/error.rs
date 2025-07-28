//! Defines the `LinkingError` enum representing possible errors
//! encountered during the linking phase of the AIPlan4Rust compilation pipeline.
//!
//! This enum aggregates errors from various subsystems involved in linking,
//! including semantic analysis, syntax tree processing, symbol table management,
//! and string interning.
//!
//! # Variants
//!
//! - [`Semantic`]: Wraps errors originating from semantic analysis.
//! - [`SyntaxTree`]: Wraps errors related to the abstract syntax tree (AST) structure.
//! - [`SemanticCheckError`]: Wraps errors encountered during semantic checks.
//! - [`SymbolTable`]: Wraps errors related to symbol table operations.
//! - [`Interner`]: Wraps errors from the string interner subsystem.
//!
//! # Usage
//!
//! This enum implements the `std::error::Error` trait via `thiserror::Error`,
//! allowing transparent error conversions and easy integration with Rust's
//! error handling ecosystem.

use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents all possible errors that can occur during the linking phase.
///
/// This enum aggregates errors from multiple components involved
/// in linking domain and problem semantic contexts, facilitating
/// unified error handling.
///
/// Each variant transparently wraps a specific error type
/// from the corresponding subsystem.
#[derive(Debug, Error)]
pub enum LinkingError {

    /// Error arising from semantic analysis failures.
    #[error(transparent)]
    Semantic(#[from] SemanticError),

    /// Error related to syntax tree construction or manipulation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Error encountered during semantic checking phase.
    #[error(transparent)]
    SemanticCheckError(#[from] SemanticCheckError),

    /// Error from symbol table operations such as lookup or insertion.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Error originating from string interner operations.
    #[error(transparent)]
    Interner(#[from] InternerError),
}
