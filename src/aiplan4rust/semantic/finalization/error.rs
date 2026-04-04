use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SemanticPassError {
    /// Errors related to the symbol table.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Errors related to the arena.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Errors related to the syntax tree.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Les données de type ou les IDs de nœuds sont absents de la déclaration.
    #[error("Finalizer: Incomplete declaration data for symbol '{symbol}'. (Type present: {has_ty}, Nodes present: {has_ids})")]
    IncompleteDeclaration {
        symbol: SymbolId,
        has_ty: bool,
        has_ids: bool,
    },

    /// Le nombre de types sémantiques ne correspond pas au nombre de nœuds AST.
    #[error("Finalizer: Type inconsistency for '{symbol}'. {ty_len} semantic types vs {ids_len} AST nodes.")]
    TypeInconsistency {
        symbol: SymbolId,
        ty_len: usize,
        ids_len: usize,
    },
}

impl SemanticPassError {
    /// Creates a [`SemanticPassError`] for a declaration missing critical data.
    #[track_caller]
    pub fn incomplete_declaration(symbol: SymbolId, has_ty: bool, has_ids: bool) -> Self {
        Self::IncompleteDeclaration {
            symbol,
            has_ty,
            has_ids,
        }
        .trace()
    }

    /// Creates a [`SemanticPassError`] for a mismatch between semantic types and AST nodes.
    #[track_caller]
    pub fn type_inconsistency(symbol: SymbolId, ty_len: usize, ids_len: usize) -> Self {
        Self::TypeInconsistency {
            symbol,
            ty_len,
            ids_len,
        }
        .trace()
    }
}

impl Traceable for SemanticPassError {}
