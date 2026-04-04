//! Defines the `LinkingError` enum representing possible errors
//! encountered during the linking phase of the AIPlan4Rust compilation pipeline.
//!
//! This enum consolidates errors from various subsystems involved in linking,
//! including semantic analysis, linking-specific checks, semantic consistency checks,
//! and symbol table operations.

use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::linking::checks::LinkingCheckError;
use crate::aiplan4rust::linking::finalization::error::SemanticFinalizationError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::passes::SemanticPassError;
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckerError;
use crate::aiplan4rust::semantic::SemanticError;
use thiserror::Error;

/// Represents all possible errors that can occur during the linking phase.
///
/// This enum aggregates errors from multiple components involved
/// in linking domain and problem semantic contexts, facilitating
/// unified error handling.
#[derive(Debug, Error)]
pub enum LinkingError {
    #[error(transparent)]
    SymbolResolver(#[from] SemanticPassError),

    #[error(transparent)]
    TypeChecker(#[from] TypeCheckerError),

    /// Error arising from semantic analysis failures.
    #[error(transparent)]
    Semantic(#[from] SemanticError),

    /// Error arising from linking-specific checks.
    #[error(transparent)]
    LinkingCheck(#[from] LinkingCheckError),

    /// Error encountered during semantic consistency checking.
    #[error(transparent)]
    SemanticCheck(#[from] SemanticCheckError),

    /// Error encountered during semantic finalization.
    #[error(transparent)]
    SemanticPass(#[from] SemanticFinalizationError),

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

    /// Occurs when the same symbol (ID + Kind) is declared more than once.
    #[error("Duplicate symbol declaration found: {0:?}")]
    DuplicateSymbolDeclaration(Symbol),
}

impl LinkingError {
    /// Returns a `LinkingError` for the case when a domain syntax tree was expected
    /// but a problem syntax tree is provided.
    #[track_caller]
    pub fn not_a_domain_syntax_tree() -> Self {
        LinkingError::NotADomainSyntaxTree.trace()
    }

    /// Returns a `LinkingError` for the case when a problem syntax tree was expected
    /// but a domain syntax tree is provided.
    #[track_caller]
    pub fn not_a_problem_syntax_tree() -> Self {
        LinkingError::NotAProblemSyntaxTree.trace()
    }

    /// Returns a `LinkingError` for hierarchical consistency mismatch between
    /// domain and problem syntax trees.
    #[track_caller]
    pub fn hierarchical_mismatch() -> Self {
        LinkingError::HierarchicalMismatch.trace()
    }

    /// Returns a `LinkingError` when the syntax tree is empty.
    #[track_caller]
    pub fn empty_syntax_tree() -> Self {
        LinkingError::EmptySyntaxTree.trace()
    }

    /// Creates a new `DuplicateSymbolDeclaration` error and captures the caller's location.
    ///
    /// This error is triggered when the linker detects that a symbol (same ID and Kind)
    /// is already present in the symbol table, which would lead to ambiguous resolution.
    ///
    /// # Parameters
    ///
    /// * `symbol` - The [`Symbol`] instance that has been declared more than once.
    ///   It contains the `SymbolId` and the `SymbolKind` to identify the collision.
    ///
    /// # Returns
    ///
    /// Returns a [`LinkingError`] of variant `DuplicateSymbolDeclaration`
    /// initialized with the provided symbol and enriched with tracing information.
    #[track_caller]
    pub fn duplicate_symbol_declaration(symbol: Symbol) -> Self {
        LinkingError::DuplicateSymbolDeclaration(symbol).trace()
    }
}

impl Traceable for LinkingError {}
