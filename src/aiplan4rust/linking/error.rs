use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

#[derive(Debug, Error)]
pub enum LinkingError {

    #[error(transparent)]
    Semantic(#[from] SemanticError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    SemanticCheckError(#[from] SemanticCheckError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    Interner(#[from] InternerError),
}
