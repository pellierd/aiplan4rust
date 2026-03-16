use std::backtrace::Backtrace;
use std::panic::Location;
use log::debug;
use thiserror::Error;

use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::semantic::{SemanticError, UnexpectedNodeKindError};
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;

#[derive(Debug, Error)]
pub enum SemanticCheckError {

    #[error(transparent)]
    Ast(#[from] AstError),

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

    #[error(transparent)]
    UnexpectedAstKind(#[from] UnexpectedNodeKindError),

    #[error("No typing declared for operand {operand_index} in binary operation at node {node_id:?}.")]
    MissingOperandType {
        node_id: NodeId,
        operand_index: usize,
    },

    #[error("No declaration found for symbol '{symbol}' in scope {scope}.")]
    MissingDeclaration {
        symbol: SymbolId,
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
        symbol: SymbolId,
        scope: Scope,
    },

    #[error("Cycle detail cannot be empty — internal inconsistency")]
    EmptyCycleDetail,

    #[error("Type index {index} for typing '{type_name}' is out of bounds (max {max})")]
    TypeIndexOutOfBounds {
        index: usize,
        type_name: SymbolId,
        max: usize,
    },

    #[error("Parent index {index} for parent typing '{parent_name}' is out of bounds (max {max})")]
    ParentIndexOutOfBounds {
        index: usize,
        parent_name: SymbolId,
        max: usize,
    },

    #[error("Object typing index {index} is out of bounds (max {max})")]
    ObjectIndexOutOfBounds {
        index: usize,
        max: usize,
    },

    #[error("Scope is empty, cannot retrieve the last scope index.")]
    EmptyScope,

    #[error("Node with index {node_id:?} not found in syntax tree during scope resolution.")]
    MissingScopeNode { node_id: NodeId },

}

impl SemanticCheckError {
    #[track_caller] // Crucial pour que Location::caller() remonte à l'appelant de cette fonction
    pub fn unexpected_ast_kind(
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    ) -> Self {
        let caller = Location::caller();

        // Préparation du message pour le log
        let msg = format!(
            "[{}:{}] Unexpected child kind for node {:?}. Expected {:?}, found {:?}",
            caller.file(),
            caller.line(),
            node_id,
            expected,
            found
        );

        // Capture du backtrace uniquement en mode debug
        #[cfg(debug_assertions)]
        {
            let bt = Backtrace::capture();
            debug!("{}\nStack backtrace:\n{}", msg, bt);
        }

        // Retourne l'erreur structurée (via votre conversion existante)
        UnexpectedNodeKindError::new(node_id, expected, found).into()
    }

    #[track_caller]
    pub fn missing_operand_type(node_id: NodeId, operand_index: usize) -> Self {
        let caller = Location::caller();

        let msg = format!(
            "[{}:{}] Missing operand typing for node {:?} at index {}",
            caller.file(),
            caller.line(),
            node_id,
            operand_index
        );

        #[cfg(debug_assertions)]
        {
            let bt = Backtrace::capture();
            debug!("{}\nStack backtrace:\n{}", msg, bt);
        }

        SemanticCheckError::MissingOperandType {
            node_id,
            operand_index
        }
    }

    pub fn missing_declaration(symbol: SymbolId, scope: Scope) -> Self {
        SemanticCheckError::MissingDeclaration { symbol, scope }
    }

    pub fn missing_declaration_arguments(scope: Scope) -> Self {
        SemanticCheckError::MissingDeclarationArguments { scope }
    }

    pub fn argument_index_out_of_bounds(index: usize, scope: Scope) -> Self {
        SemanticCheckError::ArgumentIndexOutOfBounds { index, scope }
    }

    pub fn missing_symbol_types(symbol: SymbolId, scope: Scope) -> Self {
        SemanticCheckError::MissingSymbolTypes { symbol, scope }
    }

    pub fn empty_cycle_detail() -> Self {
        SemanticCheckError::EmptyCycleDetail
    }

    pub fn type_index_out_of_bounds(index: usize, type_name: SymbolId, max: usize) -> Self {
        SemanticCheckError::TypeIndexOutOfBounds { index, type_name, max }
    }

    pub fn parent_index_out_of_bounds(index: usize, parent_name: SymbolId, max: usize) -> Self {
        SemanticCheckError::ParentIndexOutOfBounds { index, parent_name, max }
    }

    pub fn object_index_out_of_bounds(index: usize, max: usize) -> Self {
        SemanticCheckError::ObjectIndexOutOfBounds { index, max }
    }

    /// Returns a `SemanticError` indicating the scope path is empty.
    pub fn empty_scope() -> Self {
        SemanticCheckError::EmptyScope
    }

    /// Returns a `SemanticError` when a node in the scope path does not exist in the AST.
    pub fn missing_scope_node(node_id: NodeId) -> Self {
        SemanticCheckError::MissingScopeNode { node_id }
    }
}
