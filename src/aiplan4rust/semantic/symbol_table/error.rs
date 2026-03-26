//! Error definitions for the symbol table construction and resolution process.
//!
//! This module defines the `SymbolTableError` enum, which captures all the possible
//! error cases that can arise during the semantic analysis phase of symbol table building,
//! such as duplicate declarations, ambiguous symbol usage, invalid AST node types,
//! or malformed typed items.

use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolKind};
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use thiserror::Error;

/// Represents all errors that can occur during symbol table construction and resolution.
///
/// This enum covers both infrastructure failures (arena, interner) and semantic
/// violations related to symbol management (duplicates, missing declarations).
#[derive(Debug, Error)]
pub enum SymbolTableError {
    /// Error originating from the ast error.
    #[error(transparent)]
    Ast(#[from] AstError),

    /// Error originating from the syntax tree layer.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Error related to the memory arena used for allocations.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Error related to the string interner.
    #[error(transparent)]
    Interner(#[from] InternerError),

    /// Multiple declarations found for a single usage node, causing ambiguity.
    #[error("Multiple declarations found for usage at node {node_id:?}")]
    AmbiguousUsage {
        /// The AST node ID where the usage occurs.
        node_id: NodeId,

        /// All candidate declarations that matched.
        candidates: Vec<Declaration>,
    },

    /// Multiple declarations found for a symbol kind that should be unique.
    #[error("Multiple declarations found for unique symbol kind {kind:?}: {candidates:?}")]
    DuplicateDeclarationForUnique {
        /// The symbol kind that was expected to be unique.
        kind: SymbolKind,

        /// All candidate declarations found.
        candidates: Vec<Declaration>,
    },

    /// Multiple declarations found for a symbol with the same identifier and kind.
    #[error("Symbol '{ident}' of kind '{usage_kind:?}' has {count} duplicate declarations.")]
    DuplicateDeclarations {
        /// The conflicting symbol identifier.
        ident: SymbolId,

        /// The kind of the symbol.
        usage_kind: SymbolKind,

        /// Number of declarations found.
        count: usize,
    },

    /// No declaration found corresponding to a usage AST node.
    #[error("No declaration found for usage at node '{node_id}'")]
    DeclarationNotFoundForUsage {
        /// The AST node ID where the usage was expected to be declared.
        node_id: NodeId,
    },

    /// No declaration found for a symbol of the given identifier and kind in the specified scope.
    #[error("No declaration found for symbol '{symbol}' of kind '{kind:?}' in scope '{scope}'")]
    DeclarationNotFound {
        /// The identifier of the symbol.
        symbol: SymbolId,

        /// The kind of the symbol.
        kind: SymbolKind,

        /// The scope where the symbol was expected.
        scope: Scope,
    },

    /// No unique declaration found for the expected symbol kind.
    #[error("No unique declaration found for symbol kind '{kind:?}'")]
    DeclarationNotFoundForKind {
        /// The symbol kind expected to have a unique declaration.
        kind: SymbolKind,
    },

    /// Error returned when a symbol lookup fails.
    /// The `SymbolId` is included to help trace which identifier caused the issue.
    #[error("Symbol not found: {0:?}")]
    SymbolNotFound(SymbolId),
}

impl SymbolTableError {
    /// Constructs an `AmbiguousUsage` error.
    ///
    /// Used when more than one declaration is found for a usage node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the AST node where the usage occurs.
    /// * `candidates` - A list of declarations that conflict.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::AmbiguousUsage` instance.
    #[track_caller]
    pub fn ambiguous_usage(node_id: NodeId, candidates: Vec<Declaration>) -> Self {
        SymbolTableError::AmbiguousUsage {
            node_id,
            candidates,
        }
        .trace()
    }

    /// Constructs a `DuplicateUniqueDeclaration` error.
    ///
    /// Indicates a conflict where a symbol kind that must be unique (such as a `DomainName` or
    /// `ProblemName`) has been declared multiple times in the source code.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of symbol expected to have exactly one declaration.
    /// * `candidates` - A list of `Declaration` nodes representing the multiple conflicting declarations
    ///   found in the AST.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::DuplicateDeclarationForUnique` instance containing
    /// the conflicting declarations.
    #[track_caller]
    pub fn duplicated_declaration_for_unique(
        kind: SymbolKind,
        candidates: Vec<Declaration>,
    ) -> Self {
        SymbolTableError::DuplicateDeclarationForUnique { kind, candidates }.trace()
    }

    /// Constructs a `UsageNotFound` error indicating no declaration for a usage node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The AST node ID where the usage was expected.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::UsageNotFound` instance.
    #[track_caller]
    pub fn declaration_not_found_for_usage(node_id: NodeId) -> Self {
        SymbolTableError::DeclarationNotFoundForUsage { node_id }.trace()
    }

    /// Constructs a `DeclarationNotFound` error indicating no declaration found for a symbol in scope.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol identifier.
    /// * `kind` - The kind of the symbol.
    /// * `scope` - The scope where the symbol was expected.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::DeclarationNotFound` instance.
    #[track_caller]
    pub fn declaration_not_found(symbol: SymbolId, kind: SymbolKind, scope: Scope) -> Self {
        SymbolTableError::DeclarationNotFound {
            symbol,
            kind,
            scope,
        }
        .trace()
    }

    /// Constructs a `DeclarationNotFoundForKind` error indicating no unique declaration found for a kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The symbol kind expected.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::DeclarationNotFoundForKind` instance.
    #[track_caller]
    pub fn declaration_not_found_for_kind(kind: SymbolKind) -> Self {
        SymbolTableError::DeclarationNotFoundForKind { kind }.trace()
    }

    /// Creates a new `DuplicateDeclarations` error indicating that a symbol
    /// has been declared multiple times with the same identifier and kind.
    ///
    /// # Arguments
    ///
    /// * `ident` - The conflicting symbol identifier.
    /// * `usage_kind` - The kind of the symbol.
    /// * `count` - The number of declarations found.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::DuplicateDeclarations` instance.
    #[track_caller]
    pub fn duplicate_declaration(ident: SymbolId, usage_kind: SymbolKind, count: usize) -> Self {
        SymbolTableError::DuplicateDeclarations {
            ident,
            usage_kind,
            count,
        }
        .trace()
    }

    /// Creates a new `SymbolNotFound` error indicating that a lookup failed for a specific ID.
    ///
    /// # Arguments
    ///
    /// * `ident` - The identifier that could not be found in the symbol table.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::SymbolNotFound` instance with captured stack trace.
    #[track_caller]
    pub fn symbol_not_found(ident: SymbolId) -> Self {
        Self::SymbolNotFound(ident).trace()
    }
}

impl Traceable for SymbolTableError {}
