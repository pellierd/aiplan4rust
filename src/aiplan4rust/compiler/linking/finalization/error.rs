//! Error types for the AST finalization phase.
//!
//! This module defines the [`FinalizationError`] enum, which encapsulates all possible
//! failures that can occur when synchronizing the Symbol Table with the Syntax Tree.

use crate::aiplan4rust::compiler::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::compiler::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::support::lang::SymbolId;
use thiserror::Error;

/// Errors that can occur during the finalization of the semantic analysis.
///
/// This enum aggregates errors from underlying subsystems (Arena, SymbolTable, SyntaxTree)
/// and defines specific high-level errors unique to the finalization process.
#[derive(Debug, Error)]
pub enum FinalizationError {
    /// Errors forwarded from the [`SymbolTable`].
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Errors forwarded from the internal [`Arena`] storage.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Errors forwarded from the [`Tree`] structure or navigation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Occurs when a symbol's kind does not match the expected patterns for type pruning.
    ///
    /// Finalization typically expects symbols that can be part of a `TypedItem`,
    /// such as Variables, Constants, or Functions. Encountering other kinds suggests
    /// a structural mismatch between the Symbol Table and the AST.
    #[error("Finalizer: Unsupported symbol kind '{kind:?}' for symbol '{id}'. Expected Variable, Constant, PrimitiveType or Function.")]
    UnsupportedSymbolKind {
        /// The unique identifier of the offending symbol.
        id: SymbolId,
        /// The kind of the symbol that caused the mismatch.
        kind: SymbolKind,
    },
}

impl FinalizationError {
    /// Creates a new [`FinalizationError::UnsupportedSymbolKind`] from a [`Symbol`] reference.
    ///
    /// This helper automatically extracts the `id` and `kind` from the provided symbol
    /// and captures the caller's location for traceability.
    ///
    /// # Arguments
    /// * `symbol` - A reference to the symbol that triggered the error.
    #[track_caller]
    pub fn unsupported_symbol_kind(symbol: &Symbol) -> Self {
        Self::UnsupportedSymbolKind {
            id: symbol.id(),
            kind: symbol.kind(),
        }
        .trace()
    }
}

impl Traceable for FinalizationError {}
