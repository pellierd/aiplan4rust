//! Module defining semantic-related error types used throughout the semantic analysis pipeline.
//!
//! This module centralizes error definitions for semantic checking, symbol table handling,
//! type checking, and syntax tree validation.
//!
//! It includes specific error types that are shared across the semantic analysis stages:
//! - `UnexpectedNodeKindError`: Indicates an AST node has an unexpected kind.
//! - `InvalidNodeArityError`: Indicates an AST node has an invalid number of children.
//!
//! Additionally, the module provides a general enum, `SemanticError`, which wraps all
//! these specific error types to enable convenient and consistent error handling across
//! the semantic analysis pipeline.

use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::semantic::checks::SemanticCheckError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Indicates an AST node has a kind different than expected.
///
/// This error occurs when a semantic check encounters an AST node whose kind
/// does not match the expected kind(s), which typically means a structural or
/// semantic inconsistency in the AST.
///
/// # Fields
/// - `expected`: The list of acceptable AST kinds at this node.
/// - `found`: The actual kind found at the node.
/// - `node_id`: The unique identifier of the AST node in question.
#[derive(Debug, Error)]
#[error("Unexpected AST node kind at node {node_id:?}: expected {expected:?}, found {found:?}.")]
pub struct UnexpectedNodeKindError {
    expected: Vec<AstKind>,
    found: AstKind,
    node_id: NodeId,
}

impl UnexpectedNodeKindError {
    /// Creates a new `UnexpectedNodeKindError`.
    ///
    /// # Arguments
    /// - `node_id`: The identifier of the node where the mismatch was found.
    /// - `expected`: A vector of AST kinds that were expected at this node.
    /// - `found`: The actual AST kind found at this node.
    pub fn new(node_id: NodeId, expected: Vec<AstKind>, found: AstKind) -> Self {
        UnexpectedNodeKindError { node_id, expected, found }
    }
}

/// Indicates an AST node has an invalid number of children.
///
/// This error occurs when the number of children nodes does not meet the expected arity
/// for a given AST node kind, violating the language grammar or semantic rules.
///
/// # Fields
/// - `node_type`: The AST kind of the node.
/// - `node_id`: The unique identifier of the node.
/// - `child_count`: The actual number of children present.
/// - `expected_arity`: The list of allowed numbers of children for this node kind.
#[derive(Debug, thiserror::Error)]
#[error("{node_type:?} node at {node_id:?} has an unexpected number of children: {child_count}. Expected one of {expected_arity:?}.")]
pub struct InvalidNodeArityError {
    node_id: NodeId,
    node_type: AstKind,
    child_count: usize,
    expected_arity: Vec<usize>,
}

impl InvalidNodeArityError {
    /// Constructs a new `InvalidNodeArityError`.
    ///
    /// # Arguments
    /// - `node_type`: The kind of the AST node.
    /// - `node_id`: The unique identifier of the node.
    /// - `child_count`: The observed number of children nodes.
    /// - `expected_arity`: The allowed numbers of children.
    pub fn new(
        node_id: NodeId,
        node_type: AstKind,
        child_count: usize,
        expected_arity: Vec<usize>,
    ) -> Self {
        InvalidNodeArityError {
            node_id,
            node_type,
            child_count,
            expected_arity,
        }
    }
}

/// Represents all possible semantic errors that can occur during
/// parsing, analysis, and type checking phases.
///
/// This enum aggregates error types related to syntax trees, symbol tables,
/// type checking, semantic checks, and specific AST node issues.
///
/// # Variants
///
/// - `SyntaxTree`: Errors related to syntax tree construction or traversal.
/// - `SymbolTable`: Errors originating from symbol table operations.
/// - `TypeChecker`: Errors encountered during type checking phases.
/// - `SemanticCheck`: Errors raised by semantic validation and checks.
/// - `UnexpectedNodeKind`: Errors for AST nodes with an unexpected kind.
/// - `InvalidNodeArity`: Errors for AST nodes with an invalid number of children.
/// - `EmptySyntaxTree`: Error when a syntax tree is empty.
/// - `UnexpectedSyntaxTreeRootError`: Error when the syntax tree root is not a domain or problem.
#[derive(Debug, Error)]
pub enum SemanticError {
    /// Errors related to the syntax tree.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Errors from symbol table operations.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Errors during type checking.
    #[error(transparent)]
    TypeChecker(#[from] TypeCheckError),

    /// Errors raised by semantic checks.
    #[error(transparent)]
    SemanticCheck(#[from] SemanticCheckError),

    /// Errors for unexpected AST node kinds.
    #[error(transparent)]
    UnexpectedNodeKind(#[from] UnexpectedNodeKindError),

    /// Errors for invalid number of children in an AST node.
    #[error(transparent)]
    InvalidNodeArity(#[from] InvalidNodeArityError),

    /// Error related to the string interner.
    #[error(transparent)]
    Interner(#[from] InternerError),

    /// Occurs when the syntax tree is empty or missing required nodes.
    #[error("Syntax tree is empty or missing required nodes")]
    EmptySyntaxTree,

    /// Occurs when the syntax tree root is neither a domain nor a problem.
    #[error("Syntax tree root is invalid: expected a domain or a problem")]
    UnexpectedSyntaxTreeRootError,
}

impl SemanticError {
    /// Creates a `SemanticError` for an AST node that has an unexpected kind.
    ///
    /// # Parameters
    /// - `node_id`: The unique identifier of the AST node where the mismatch occurred.
    /// - `expected`: A vector of AST node kinds that were expected at this node.
    /// - `found`: The actual AST node kind found at this node.
    ///
    /// # Returns
    /// A `SemanticError` wrapping an `UnexpectedNodeKindError`.
    pub fn unexpected_ast_kind(
        node_id: NodeId,
        expected: Vec<AstKind>,
        found: AstKind,
    ) -> Self {
        SemanticError::UnexpectedNodeKind(
            UnexpectedNodeKindError::new(node_id, expected, found)
        )
    }

    /// Creates a `SemanticError` for an AST node that has an invalid number of children.
    ///
    /// # Parameters
    /// - `node_id`: The unique identifier of the AST node.
    /// - `node_type`: The kind of the AST node.
    /// - `child_count`: The actual number of children present at this node.
    /// - `expected_arity`: A list of allowed numbers of children for this node kind.
    ///
    /// # Returns
    /// A `SemanticError` wrapping an `InvalidNodeArityError`.
    pub fn invalid_node_arity(
        node_id: NodeId,
        node_type: AstKind,
        child_count: usize,
        expected_arity: Vec<usize>,
    ) -> Self {
        SemanticError::InvalidNodeArity(
            InvalidNodeArityError::new(node_id, node_type, child_count, expected_arity)
        )
    }

    /// Returns a `SemanticError` when the syntax tree is empty or missing required nodes.
    ///
    /// # Returns
    /// A `SemanticError` variant `EmptySyntaxTree`.
    pub fn empty_syntax_tree() -> Self {
        SemanticError::EmptySyntaxTree
    }

    /// Returns a `SemanticError` when the syntax tree root is not a domain or a problem.
    ///
    /// # Returns
    /// A `SemanticError` variant `UnexpectedSyntaxTreeRootError`.
    pub fn unexpected_syntax_tree_root() -> Self {
        SemanticError::UnexpectedSyntaxTreeRootError
    }
}
