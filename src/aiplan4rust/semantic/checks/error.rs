use thiserror::Error;

use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope};
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Represents errors that can occur during symbol table construction or resolution.
#[derive(Debug, Error)]
pub enum SemanticCheckError {
    /// Generic internal error with a descriptive message.
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Arena(#[from] ArenaError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    TypeChecker(#[from] TypeCheckError),

    #[error(transparent)]
    Interner(#[from] InternerError),


    #[error("Unexpected AST node kind at node {node_id:?}: expected {expected:?}, found {found:?}.")]
    UnexpectedAstKind {
        expected: AstKind,
        found: AstKind,
        node_id: NodeId,
    },

    #[error("No type declared for operand {operand_index} in binary operation at node {node_id:?}.")]
    MissingOperandType {
        node_id: NodeId,
        operand_index: usize,
    },

    #[error("No declaration found for symbol '{symbol}' in scope {scope}.")]
    MissingDeclaration {
        symbol: Ident,
        scope: Scope,
    },

    #[error("Failed to retrieve arguments for declaration in scope {scope}.")]
    MissingDeclarationArguments {
        scope: Scope,
    },

    #[error("Argument index {index} out of bounds for declaration in scope {scope}.")]
    ArgumentIndexOutOfBounds {
        index: usize,
        scope: Scope,
    },

    #[error("Failed to retrieve types for symbol '{symbol}' in scope {scope}.")]
    MissingSymbolTypes {
        symbol: Ident,
        scope: Scope,
    },

    #[error("Cycle detail cannot be empty — internal inconsistency")]
    EmptyCycleDetail,

    #[error("Type index {index} for type '{type_name}' is out of bounds (max {max})")]
    TypeIndexOutOfBounds {
        index: usize,
        type_name: Ident,
        max: usize,
    },

    #[error("Parent index {index} for parent type '{parent_name}' is out of bounds (max {max})")]
    ParentIndexOutOfBounds {
        index: usize,
        parent_name: Ident,
        max: usize,
    },

    #[error("Object type index {index} is out of bounds (max {max})")]
    ObjectIndexOutOfBounds {
        index: usize,
        max: usize,
    },
}

impl SemanticCheckError {
    /// Helper to create an `InternalError` from any displayable message.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        SemanticCheckError::InternalError(msg.into())
    }

    pub fn unexpected_ast_kind(expected: AstKind, found: AstKind, node_id: NodeId) -> Self {
        SemanticCheckError::UnexpectedAstKind { expected, found, node_id }
    }

    pub fn missing_operand_type(node_id: NodeId, operand_index: usize) -> Self {
        SemanticCheckError::MissingOperandType { node_id, operand_index }
    }
    pub fn missing_declaration(symbol: Ident, scope: Scope) -> Self {
        SemanticCheckError::MissingDeclaration { symbol, scope }
    }

    pub fn missing_declaration_arguments(scope: Scope) -> Self {
        SemanticCheckError::MissingDeclarationArguments { scope }
    }

    pub fn argument_index_out_of_bounds(index: usize, scope: Scope) -> Self {
        SemanticCheckError::ArgumentIndexOutOfBounds { index, scope }
    }

    pub fn missing_symbol_types(symbol: Ident, scope: Scope) -> Self {
        SemanticCheckError::MissingSymbolTypes { symbol, scope }
    }

    pub fn empty_cycle_detail() -> Self {
        SemanticCheckError::EmptyCycleDetail
    }

    pub fn type_index_out_of_bounds(index: usize, type_name: Ident, max: usize) -> Self {
        SemanticCheckError::TypeIndexOutOfBounds { index, type_name, max }
    }

    pub fn parent_index_out_of_bounds(index: usize, parent_name: Ident, max: usize) -> Self {
        SemanticCheckError::ParentIndexOutOfBounds { index, parent_name, max }
    }

    pub fn object_index_out_of_bounds(index: usize, max: usize) -> Self {
        SemanticCheckError::ObjectIndexOutOfBounds { index, max }
    }
}
