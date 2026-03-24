//! Module defining semantic-related error types used throughout the semantic analysis pipeline.
//!
//! This module centralizes error definitions for semantic checking, symbol table handling,
//! typing checking, and syntax tree validation.
//!
//! It includes specific error types that are shared across the semantic analysis stages:
//! - `UnexpectedNodeKindError`: Indicates an AST node has an unexpected kind.
//! - `InvalidNodeArityError`: Indicates an AST node has an invalid number of children.
//!
//! Additionally, the module provides a general enum, `SemanticError`, which wraps all
//! these specific error types to enable convenient and consistent error handling across
//! the semantic analysis pipeline.

use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::finalization::error::FinalizationError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use thiserror::Error;

/// Represents all possible semantic errors that can occur during
/// parsing, analysis, and typing checking phases.
///
/// This enum aggregates error types related to syntax trees, symbol tables,
/// typing checking, semantic checks, and specific AST node issues.
///
/// # Variants
///
/// - `Ast`: Errors related to the core AST structure.
/// - `Arena`: Errors originating from the node storage arena.
/// - `SyntaxTree`: Errors related to syntax tree construction or traversal.
/// - `SymbolTable`: Errors originating from symbol table operations.
/// - `TypeChecker`: Errors encountered during typing checking phases.
/// - `SemanticCheck`: Errors raised by specialized semantic validation passes.
/// - `Interner`: Errors related to string interning and symbol ID retrieval.
/// - `EmptySyntaxTree`: Error when a syntax tree is empty or missing nodes.
/// - `UnexpectedSyntaxTreeRootError`: Error when the root node is not a Domain or Problem.
/// - `UnexpectedNodeKind`: Errors for AST nodes with an unexpected kind.
/// - `InvalidNodeArity`: Errors for AST nodes with an incorrect number of children.
#[derive(Debug, Error)]
pub enum SemanticError {
    /// Errors related to the ast.
    #[error(transparent)]
    Ast(#[from] AstError),

    /// Errors related to the arena.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// Errors related to the syntax tree.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Errors from symbol table operations.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Errors during typing checking.
    #[error(transparent)]
    TypeChecker(#[from] TypeCheckError),

    /// Errors raised by semantic checks.
    #[error(transparent)]
    SemanticCheck(#[from] SemanticCheckError),

    /// Errors related to the finalization.
    #[error(transparent)]
    Finalization(#[from] FinalizationError),

    /// Error related to the string interner.
    #[error(transparent)]
    Interner(#[from] InternerError),

    /// Occurs when the syntax tree is empty or missing required nodes.
    #[error("Syntax tree is empty or missing required nodes")]
    EmptySyntaxTree,

    /// Occurs when the syntax tree root is neither a domain nor a problem.
    #[error("Syntax tree root is invalid: expected a domain or a problem")]
    UnexpectedSyntaxTreeRootError,

    /// Occurs when a node has a kind that doesn't match the grammar's expectations.
    #[error(
        "Unexpected AST node kind at node {node_id:?}: expected {expected:?}, found {found:?}."
    )]
    UnexpectedNodeKind {
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    },

    /// Occurs when a node has an incorrect number of children.
    #[error("{node_type:?} node at {node_id:?} has an unexpected number of children: {child_count}. Expected one of {expected_arity:?}.")]
    InvalidNodeArity {
        node_id: NodeId,
        node_type: AstKind,
        child_count: usize,
        expected_arity: Vec<usize>,
    },
}

impl SemanticError {
    /// Creates a [`SemanticError`] for an AST node that has an unexpected kind.
    ///
    /// # Parameters
    /// * `node_id` - The unique identifier of the AST node where the mismatch occurred.
    /// * `expected` - A vector of AST node kinds that were expected at this node.
    /// * `found` - The actual AST node kind found at this node.
    #[track_caller]
    pub fn unexpected_node_kind(node_id: NodeId, expected: Vec<AstKind>, found: AstKind) -> Self {
        Self::UnexpectedNodeKind {
            node_id,
            expected,
            found,
        }
        .trace()
    }

    /// Creates a [`SemanticError`] for an AST node that has an invalid number of children.
    ///
    /// # Parameters
    /// * `node_id` - The unique identifier of the AST node.
    /// * `node_type` - The kind of the AST node.
    /// * `child_count` - The actual number of children present at this node.
    /// * `expected_arity` - A list of allowed numbers of children for this node kind.
    #[track_caller]
    pub fn invalid_node_arity(
        node_id: NodeId,
        node_type: AstKind,
        child_count: usize,
        expected_arity: Vec<usize>,
    ) -> Self {
        Self::InvalidNodeArity {
            node_id,
            node_type,
            child_count,
            expected_arity,
        }
        .trace()
    }

    /// Returns a `SemanticError` when the syntax tree is empty or missing required nodes.
    ///
    /// # Returns
    /// A `SemanticError` variant `EmptySyntaxTree`.
    #[track_caller]
    pub fn empty_syntax_tree() -> Self {
        SemanticError::EmptySyntaxTree.trace()
    }

    /// Returns a `SemanticError` when the syntax tree root is not a domain or a problem.
    ///
    /// # Returns
    /// A `SemanticError` variant `UnexpectedSyntaxTreeRootError`.
    #[track_caller]
    pub fn unexpected_syntax_tree_root() -> Self {
        SemanticError::UnexpectedSyntaxTreeRootError.trace()
    }
}

impl Traceable for SemanticError {}
