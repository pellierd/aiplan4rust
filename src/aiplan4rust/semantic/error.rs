use thiserror::Error;

use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

#[derive(Debug, Error)]
#[error("Unexpected AST node kind at node {node_id:?}: expected {expected:?}, found {found:?}.")]
pub struct UnexpectedAstKindError {
    expected: Vec<AstKind>,
    found: AstKind,
    node_id: NodeId,
}

impl UnexpectedAstKindError {
    pub fn new(node_id: NodeId, expected: Vec<AstKind>, found: AstKind) -> Self {
        UnexpectedAstKindError { node_id, expected, found }
    }
}

#[derive(Debug, Error)]
pub enum SemanticError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    TypeChecker(#[from] TypeCheckError),

    #[error(transparent)]
    SemanticCheck(#[from] SemanticCheckError),

    #[error(transparent)]
    UnexpectedAstKind(#[from] UnexpectedAstKindError),
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
        SemanticError::UnexpectedAstKind(
            UnexpectedAstKindError::new(node_id, expected, found)
        )
    }
}
