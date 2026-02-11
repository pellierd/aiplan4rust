//! Error definitions for the symbol table construction and resolution process.
//!
//! This module defines the `SymbolTableError` enum, which captures all the possible
//! error cases that can arise during the semantic analysis phase of symbol table building,
//! such as duplicate declarations, ambiguous symbol usage, invalid AST node types,
//! or malformed typed items.

use thiserror::Error;

use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::StringID;
use crate::aiplan4rust::semantic::error::InvalidNodeArityError;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolKind};
use crate::aiplan4rust::semantic::UnexpectedNodeKindError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;

/// Represents all the errors that can occur during symbol table construction and resolution.
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
        ident: StringID,

        /// The kind of the symbol.
        usage_kind: SymbolKind,

        /// Number of declarations found.
        count: usize,
    },

    /// The AST node kind does not match the expected kinds.
    #[error(transparent)]
    UnexpectedNodeKind(#[from] UnexpectedNodeKindError),

    /// The AST node has an incorrect number of children.
    #[error(transparent)]
    InvalidNodeArity(#[from] InvalidNodeArityError),

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
        symbol: StringID,

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
    pub fn ambiguous_usage(node_id: NodeId, candidates: Vec<Declaration>) -> Self {
        SymbolTableError::AmbiguousUsage { node_id, candidates }
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
    pub fn duplicated_declaration_for_unique(
        kind: SymbolKind,
        candidates: Vec<Declaration>,
    ) -> Self {
        SymbolTableError::DuplicateDeclarationForUnique { kind, candidates }
    }

    /// Constructs an `UnexpectedNodeKind` error from a node ID, expected kinds, and actual kind.
    ///
    /// Delegates to the common `UnexpectedNodeKindError` structure.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node with the wrong kind.
    /// * `expected` - The list of expected kinds.
    /// * `found` - The actual kind found in the node.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::UnexpectedNodeKind` instance.
    pub fn unexpected_node_kind(
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    ) -> Self {
        UnexpectedNodeKindError::new(node_id, expected, found).into()
    }

    /// Constructs an `InvalidNodeArity` error from a node ID, node kind, child count,
    /// and the list of acceptable arities.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node with the incorrect number of children.
    /// * `node_type` - The kind of the AST node.
    /// * `child_count` - The actual number of children found.
    /// * `expected_arity` - The list of acceptable numbers of children.
    ///
    /// # Returns
    ///
    /// A new `SymbolTableError::InvalidNodeArity` instance.
    pub fn invalid_node_arity(
        node_id: NodeId,
        node_type: AstKind,
        child_count: usize,
        expected_arity: Vec<usize>,
    ) -> Self {
        InvalidNodeArityError::new(
            node_id,
            node_type,
            child_count,
            expected_arity).into()
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
        let caller = std::panic::Location::caller();
        let err = SymbolTableError::DeclarationNotFoundForUsage { node_id };

        if log::log_enabled!(log::Level::Debug) {
            let bt = std::backtrace::Backtrace::force_capture();

            log::debug!(
            "\nError at {}:{}:{}\n{}\nStack trace:\n{}",
            caller.file(),
            caller.line(),
            caller.column(),
            err,
            bt
        );
        }

        err
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
    pub fn declaration_not_found(symbol: StringID, kind: SymbolKind, scope: Scope) -> Self {
        SymbolTableError::DeclarationNotFound { symbol, kind, scope }
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
    pub fn declaration_not_found_for_kind(kind: SymbolKind) -> Self {
        SymbolTableError::DeclarationNotFoundForKind { kind }
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
    pub fn duplicate_declaration(
        ident: StringID,
        usage_kind: SymbolKind,
        count: usize,
    ) -> Self {
        SymbolTableError::DuplicateDeclarations {
            ident,
            usage_kind,
            count,
        }
    }
}
