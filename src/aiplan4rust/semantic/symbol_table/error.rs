use thiserror::Error;

use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolEntry, SymbolKind};
use crate::aiplan4rust::semantic::UnexpectedAstKindError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Represents errors that can occur during symbol table construction or resolution.
#[derive(Debug, Error)]
pub enum SymbolTableError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Multiple declarations found for a single usage node, which is ambiguous.
    ///
    /// This error occurs when more than one declaration matches the scope constraints
    /// for a usage, indicating an invalid or ambiguous resolution state.
    #[error("Multiple declarations found for usage at node {node_id:?}")]
    MultipleDeclarationsForUsage {
        /// The AST node ID where the usage occurs.
        node_id: NodeId,

        /// All candidate declarations that matched.
        candidates: Vec<Declaration>,
    },

    #[error("Multiple declarations found for unique symbol kind {kind:?}: {candidates:?}")]
    NonUniqueSymbolDeclaration {
        kind: SymbolKind,
        candidates: Vec<SymbolEntry>,
    },

    #[error("Symbol '{ident}' of kind '{usage_kind:?}' has {count} declarations, which is invalid.")]
    MultipleSymbolDeclarations {
        ident: Ident,
        usage_kind: SymbolKind,
        count: usize,
    },

    #[error(transparent)]  // Utilisation de l'erreur commune UnexpectedAstKindError
    UnexpectedAstKind(#[from] UnexpectedAstKindError),

    #[error("TypedItem at node {node_id:?} has an unexpected number of children: {child_count}. Expected 1 or 2.")]
    InvalidTypedItemArity {
        node_id: NodeId,
        child_count: usize,
    }
}

impl SymbolTableError {
    /// Helper to create an `InternalError` from any displayable message.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        SymbolTableError::InternalError(msg.into())
    }

    /// Helper to create a `MultipleDeclarationsForUsage` error.
    pub fn multiple_declarations(node_id: NodeId, candidates: Vec<Declaration>) -> Self {
        SymbolTableError::MultipleDeclarationsForUsage { node_id, candidates }
    }

    pub fn non_unique_symbol_declaration(
        kind: SymbolKind,
        candidates: Vec<SymbolEntry>,
    ) -> Self {
        SymbolTableError::NonUniqueSymbolDeclaration { kind, candidates }
    }

    pub fn unexpected_ast_kind(
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    ) -> Self {
        UnexpectedAstKindError::new(node_id, expected, found).into()
    }

    pub fn invalid_typed_item_arity(node_id: NodeId, child_count: usize) -> Self {
        SymbolTableError::InvalidTypedItemArity { node_id, child_count }
    }
}
