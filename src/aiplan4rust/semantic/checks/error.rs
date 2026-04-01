//! Semantic analysis error definitions for PDDL/HDDL components.
//!
//! This module defines the [`SemanticCheckError`] enum, which serves as the unified error
//! type for all semantic validation passes. It encapsulates errors originating from
//! lower-level components (AST, Syntax Tree, Symbol Table) while introducing
//! specific variants for logical inconsistencies in the planning domain.

use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckerError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use thiserror::Error;

/// Represents all possible errors that can occur during the semantic analysis phase.
///
/// This enum uses `#[error(transparent)]` for internal infrastructure errors to
/// preserve the original context, and provides detailed, human-readable messages
/// for domain-specific semantic violations.
#[derive(Debug, Error)]
pub enum SemanticCheckError {
    // --- Infrastructure Error Wrappers ---
    /// Errors related to AST construction or integrity.
    #[error(transparent)]
    Ast(#[from] AstError),

    /// Errors occurring during syntax tree traversal or node access.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Errors originating from the internal arena memory management.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Errors encountered while querying or modifying the symbol table.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Errors produced by the type checking engine during expression validation.
    #[error(transparent)]
    TypeChecker(#[from] TypeCheckerError),

    /// Errors occurring during symbol interning or string retrieval.
    #[error(transparent)]
    Interner(#[from] InternerError),

    // --- Structural Semantic Errors ---
    /// Triggered when an operand in a binary operation (e.g., comparison, arithmetic)
    /// lacks a resolvable type.
    #[error(
        "No typing declared for operand {operand_index} in binary operation at node {node_id:?}."
    )]
    MissingOperandType {
        /// The unique identifier of the operation node.
        node_id: NodeId,
        /// The zero-based index of the problematic operand.
        operand_index: usize,
    },

    // --- Symbol & Scope Resolution Errors ---
    /// Triggered when a symbol is used but no corresponding declaration exists in the
    /// current or parent scopes.
    #[error("No declaration found for symbol '{symbol}' in scope {scope}.")]
    MissingDeclaration {
        /// The identifier of the unresolved symbol.
        symbol: SymbolId,
        /// The scope path where resolution failed.
        scope: Scope,
    },

    /// Internal error: the declaration exists but its argument metadata is missing.
    #[error("Failed to retrieve arguments for declaration in scope {scope}.")]
    MissingDeclarationArguments {
        /// The scope path of the declaration.
        scope: Scope,
    },

    // --- Typing & Hierarchy Errors ---
    /// Critical internal error when a type cycle is detected but no details are provided.
    #[error("Cycle detail cannot be empty — internal inconsistency")]
    EmptyCycleDetail,

    /// Error during type resolution where a type index refers to a non-existent entry.
    #[error("Type index {index} for typing '{type_name}' is out of bounds (max {max})")]
    TypeIndexOutOfBounds {
        /// The out-of-bounds index.
        index: usize,
        /// The name of the type being processed.
        type_name: SymbolId,
        /// The maximum allowed index.
        max: usize,
    },

    /// Error during parent type resolution in a `:typing` hierarchy.
    #[error("Parent index {index} for parent typing '{parent_name}' is out of bounds (max {max})")]
    ParentIndexOutOfBounds {
        /// The out-of-bounds index.
        index: usize,
        /// The name of the parent type.
        parent_name: SymbolId,
        /// The maximum allowed index.
        max: usize,
    },

    /// Error during object type mapping, usually related to constant or object definitions.
    #[error("Object typing index {index} is out of bounds (max {max})")]
    ObjectIndexOutOfBounds {
        /// The out-of-bounds index.
        index: usize,
        /// The maximum allowed index.
        max: usize,
    },

    // --- Contextual & Tree Resolution Errors ---
    /// Occurs when an operation requires a non-empty scope path to function.
    #[error("Scope is empty, cannot retrieve the last scope index.")]
    EmptyScope,

    /// Occurs when a node ID referenced in a scope path does not exist in the syntax tree.
    #[error("Node with index {node_id:?} not found in syntax tree during scope resolution.")]
    MissingScopeNode {
        /// The missing node identifier.
        node_id: NodeId,
    },
}

impl SemanticCheckError {
    /// Creates a [`MissingOperandType`](Self::MissingOperandType) error.
    ///
    /// * `node_id`: The ID of the binary operation node.
    /// * `operand_index`: The index (0 or 1) of the operand missing a type.
    #[track_caller]
    pub fn missing_operand_type(node_id: NodeId, operand_index: usize) -> Self {
        Self::MissingOperandType {
            node_id,
            operand_index,
        }
        .trace()
    }

    /// Creates a [`MissingDeclaration`](Self::MissingDeclaration) error.
    ///
    /// * `symbol`: The identifier of the symbol that couldn't be resolved.
    /// * `scope`: The scope path where the resolution was attempted.
    #[track_caller]
    pub fn missing_declaration(symbol: SymbolId, scope: Scope) -> Self {
        Self::MissingDeclaration { symbol, scope }.trace()
    }

    /// Creates a [`MissingDeclarationArguments`](Self::MissingDeclarationArguments) error.
    ///
    /// * `scope`: The scope of the declaration whose arguments are missing.
    #[track_caller]
    pub fn missing_declaration_arguments(scope: Scope) -> Self {
        Self::MissingDeclarationArguments { scope }.trace()
    }

    /// Creates an [`EmptyCycleDetail`](Self::EmptyCycleDetail) error.
    #[track_caller]
    pub fn empty_cycle_detail() -> Self {
        Self::EmptyCycleDetail.trace()
    }

    /// Creates a [`TypeIndexOutOfBounds`](Self::TypeIndexOutOfBounds) error.
    ///
    /// * `index`: The invalid type index.
    /// * `type_name`: The identifier of the type being resolved.
    /// * `max`: The maximum valid index.
    #[track_caller]
    pub fn type_index_out_of_bounds(index: usize, type_name: SymbolId, max: usize) -> Self {
        Self::TypeIndexOutOfBounds {
            index,
            type_name,
            max,
        }
        .trace()
    }

    /// Creates a [`ParentIndexOutOfBounds`](Self::ParentIndexOutOfBounds) error.
    ///
    /// * `index`: The invalid parent index.
    /// * `parent_name`: The identifier of the parent type.
    /// * `max`: The maximum valid index.
    #[track_caller]
    pub fn parent_index_out_of_bounds(index: usize, parent_name: SymbolId, max: usize) -> Self {
        Self::ParentIndexOutOfBounds {
            index,
            parent_name,
            max,
        }
        .trace()
    }

    /// Creates an [`ObjectIndexOutOfBounds`](Self::ObjectIndexOutOfBounds) error.
    ///
    /// * `index`: The invalid object typing index.
    /// * `max`: The maximum valid index.
    #[track_caller]
    pub fn object_index_out_of_bounds(index: usize, max: usize) -> Self {
        Self::ObjectIndexOutOfBounds { index, max }.trace()
    }

    /// Creates an [`EmptyScope`](Self::EmptyScope) error.
    #[track_caller]
    pub fn empty_scope() -> Self {
        Self::EmptyScope.trace()
    }

    /// Creates a [`MissingScopeNode`](Self::MissingScopeNode) error.
    ///
    /// * `node_id`: The identifier of the node missing from the syntax tree.
    #[track_caller]
    pub fn missing_scope_node(node_id: NodeId) -> Self {
        Self::MissingScopeNode { node_id }.trace()
    }
}

impl Traceable for SemanticCheckError {}
