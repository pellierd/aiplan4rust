//! Defines the `LinkingError` enum representing possible errors
//! encountered during the linking phase of the AIPlan4Rust compilation pipeline.
//!
//! This enum consolidates errors from various subsystems involved in linking,
//! including semantic analysis, linking-specific checks, semantic consistency checks,
//! and symbol table operations.
//!
//! # Variants
//!
//! - [`Semantic`]: Wraps errors from the core semantic analysis phase.
//! - [`LinkingCheck`]: Wraps errors arising from linking-specific validations.
//! - [`SemanticCheckError`]: Wraps errors from semantic consistency checks.
//! - [`SymbolTable`]: Wraps errors related to symbol table operations.
//!
//! # Integration
//!
//! This enum implements the `std::error::Error` trait via `thiserror::Error`,
//! enabling seamless error composition and propagation within Rust's
//! error handling ecosystem.

use thiserror::Error;
use crate::aiplan4rust::linking::checks::LinkingCheckError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

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

    /// Error arising from linking-specific checks.
    #[error(transparent)]
    LinkingCheck(#[from] LinkingCheckError),

    /// Error encountered during semantic consistency checking.
    #[error(transparent)]
    SemanticCheckError(#[from] SemanticCheckError),

    /// Error from symbol table operations such as lookup or insertion.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),
}
