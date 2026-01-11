//! Defines the `LinkingError` enum representing possible errors
//! encountered during the linking phase of the AIPlan4Rust compilation pipeline.
//!
//! This enum consolidates errors from various subsystems involved in linking,
//! including semantic analysis, linking-specific checks, semantic consistency checks,
//! and symbol table operations.

use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::remap_idents::RemapIdentError;
use crate::aiplan4rust::linking::checks::LinkingCheckError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

/// Represents all possible errors that can occur during the linking phase.
///
/// This enum aggregates errors from multiple components involved
/// in linking domain and problem semantic contexts, facilitating
/// unified error handling.
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

    /// Error related to the string interner.
    #[error(transparent)]
    Interner(#[from] InternerError),

    /// Occurs when a domain syntax tree was expected but a problem syntax tree was provided.
    #[error("Expected a domain syntax tree, but got a problem syntax tree")]
    NotADomainSyntaxTree,

    /// Occurs when a problem syntax tree was expected but a domain syntax tree was provided.
    #[error("Expected a problem syntax tree, but got a domain syntax tree")]
    NotAProblemSyntaxTree,

    /// Occurs when a hierarchical consistency check fails between domain and problem syntax trees.
    #[error("Domain and problem syntax trees are not consistent in hierarchical structure")]
    HierarchicalMismatch,

    /// Occurs when the syntax tree is empty or missing required nodes.
    #[error("Syntax tree is empty or missing required nodes")]
    EmptySyntaxTree,

    /// Wraps any RemapIdentError encountered
    #[error(transparent)]
    RemapIndent(#[from] RemapIdentError),
}

impl LinkingError {
    /// Returns a `LinkingError` for the case when a domain syntax tree was expected
    /// but a problem syntax tree is provided.
    pub fn not_a_domain_syntax_tree() -> Self {
        LinkingError::NotADomainSyntaxTree
    }

    /// Returns a `LinkingError` for the case when a problem syntax tree was expected
    /// but a domain syntax tree is provided.
    pub fn not_a_problem_syntax_tree() -> Self {
        LinkingError::NotAProblemSyntaxTree
    }

    /// Returns a `LinkingError` for hierarchical consistency mismatch between
    /// domain and problem syntax trees.
    pub fn hierarchical_mismatch() -> Self {
        LinkingError::HierarchicalMismatch
    }

    /// Returns a `LinkingError` when the syntax tree is empty.
    pub fn empty_syntax_tree() -> Self {
        LinkingError::EmptySyntaxTree
    }
}
