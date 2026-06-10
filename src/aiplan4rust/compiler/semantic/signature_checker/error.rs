//! Error Handling for the Signature Matching Engine
//!
//! This module defines the [`SignatureMatcherError`] enum, which consolidates
//! all possible failures that can occur during the signature validation process.
//!
//! It acts as a bridge between the different semantic layers (AST, Symbol Table,
//! and Type Checker), providing a unified error type that supports transparent
//! propagation and detailed tracing.

use crate::aiplan4rust::compiler::semantic::symbol::declaration::DeclarationError;
use crate::aiplan4rust::compiler::semantic::type_checker::TypeCheckerError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::compiler::syntax::ast::tree::NodeId;
use crate::aiplan4rust::compiler::syntax::ast::AstError;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// Enumerates the errors that can arise during signature matching and symbol resolution.
#[derive(Debug, Error)]
pub enum SignatureMatcherError {
    /// Occurs when an argument used at a call site does not correspond
    /// to any known declaration in the local or domain tables.
    #[error("Failed to resolve argument '{arg}' during signature matching.")]
    UnresolvedArgument {
        /// The identifier of the AST node that could not be resolved.
        arg: NodeId,
    },

    /// Occurs when the category of a symbol (Kind) is fundamentally
    /// incompatible with the expected context.
    #[error("The symbol kind is invalid for this context")]
    InvalidSymbolKind,

    /// Transparent propagation of AST access errors (e.g., node retrieval failures).
    #[error(transparent)]
    Ast(#[from] AstError),

    /// Transparent propagation of symbol declaration errors.
    #[error(transparent)]
    Declaration(#[from] DeclarationError),

    /// Transparent propagation of semantic type-checking errors.
    #[error(transparent)]
    TypeChecker(#[from] TypeCheckerError),

    /// Transparent propagation of low-level syntax tree errors.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),
}

impl SignatureMatcherError {
    /// Creates a traced error for an argument that cannot be resolved.
    ///
    /// # Arguments
    /// * `node_id` - The ID of the argument node that failed resolution.
    pub fn unresolved_argument(node_id: NodeId) -> Self {
        Self::UnresolvedArgument { arg: node_id }.trace()
    }

    /// Creates a traced error for an invalid symbol kind detection.
    pub fn invalid_symbol_kind() -> Self {
        Self::InvalidSymbolKind.trace()
    }
}

impl Traceable for SignatureMatcherError {}
