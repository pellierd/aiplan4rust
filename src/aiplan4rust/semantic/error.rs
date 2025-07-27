use thiserror::Error;

use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

#[derive(Debug, Error)]
pub enum SemanticError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SyntaxTee(#[from] SyntaxTreeError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    TypeChecker(#[from] TypeCheckError),

    #[error(transparent)]
    SemanticCheck(#[from] SemanticCheckError),

    #[error("Unexpected AST node kind at node {node_id:?}: expected {expected:?}, found {found:?}.")]
    UnexpectedAstKind {
        expected: Vec<AstKind>,
        found: AstKind,
        node_id: NodeId,
    },
}

impl SemanticError {
    /// Helper to create an `InternalError` from any displayable message.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        SemanticError::InternalError(msg.into())
    }

    pub fn unexpected_ast_kind(
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    ) -> Self {
        SemanticError::UnexpectedAstKind {
            expected,
            found,
            node_id,
        }
    }
}
