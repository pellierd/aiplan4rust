//! Module defining the error types used during parsing and AST construction.
//!
//! This module contains the `ParseContextError` enum, which represents all
//! possible errors that can occur while working with the Abstract Syntax Tree (AST)
//! or its related parsing context.
//!
//! The error type encapsulates lower-level issues such as arena allocation failures,
//! syntax tree errors, and other internal logic errors that may arise during AST
//! construction or transformation.
//!
//! Errors from the arena allocator and syntax tree are wrapped to provide
//! a unified error interface for higher-level parsing logic.

use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

/// Represents all possible errors that can occur while working with the AST
/// or its related parsing context.
///
/// This error type encapsulates lower-level issues such as arena allocation failures,
/// syntax tree errors, as well as internal logic errors that may arise during AST
/// construction or transformation.
#[derive(Error, Debug)]
pub enum ParseContextError {
    /// An error occurred during arena allocation or manipulation.
    ///
    /// This usually indicates a failure in allocating or retrieving nodes
    /// from the AST arena data structure.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error related to the syntax tree structure or its operations.
    #[error("Syntax tree error: {0}")]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Une section a été définie plus d'une fois (ex: Predicates ou Functions)
    #[error("Duplicate section: {0:?}")]
    DuplicateSection(AstKind),

    /// L'ordre des éléments dans le domaine n'est pas respecté
    #[error("Invalid order: {0}")]
    InvalidOrder(&'static str),
}

impl ParseContextError {
    // Crée une erreur pour une section dupliquée
    pub fn duplicate_section(kind: AstKind) -> Self {
        ParseContextError::DuplicateSection(kind)
    }

    /// Crée une erreur pour un ordre invalide
    pub fn invalid_order(msg: &'static str) -> Self {
        ParseContextError::InvalidOrder(msg)
    }
}
