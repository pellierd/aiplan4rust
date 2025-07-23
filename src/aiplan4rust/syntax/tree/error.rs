
use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::syntax::ast::AstError;

/// Errors that can occur when working with the AST.
#[derive(Error, Debug)]
pub enum SyntaxTreeError {

    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    #[error("Expected Ident, but content was not an Ident")]
    NotAnIdent,

    #[error("Expected Float, but content was not a Float")]
    NotAFloat,

    #[error("Expected Binary comparison operator, but content was not one")]
    NotABinaryComp,

    #[error("Expected Assignment operator, but content was not one")]
    NotAnAssignOp,

    #[error("Expected Arithmetic operator, but content was not one")]
    NotAnArithmeticOp,

    #[error("Expected Optimization directive, but content was not one")]
    NotAnOptimization,

    #[error("Expected SymbolRef, but content was not a SymbolRef")]
    NotASymbolRef,

    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl SyntaxTreeError {
    /// Crée une erreur interne personnalisée.
    pub fn internal_error(msg: impl Into<String>) -> Self {
        SyntaxTreeError::InternalError(msg.into())
    }

    /// Crée une erreur pour contenu non identifiant.
    pub fn not_an_ident() -> Self {
        SyntaxTreeError::NotAnIdent
    }

    /// Crée une erreur pour contenu non float.
    pub fn not_a_float() -> Self {
        SyntaxTreeError::NotAFloat
    }

    /// Crée une erreur pour contenu non opérateur de comparaison.
    pub fn not_a_binary_comp() -> Self {
        SyntaxTreeError::NotABinaryComp
    }

    /// Crée une erreur pour contenu non opérateur d’assignement.
    pub fn not_an_assign_op() -> Self {
        SyntaxTreeError::NotAnAssignOp
    }

    /// Crée une erreur pour contenu non opérateur arithmétique.
    pub fn not_an_arithmetic_op() -> Self {
        SyntaxTreeError::NotAnArithmeticOp
    }

    /// Crée une erreur pour contenu non directive d’optimisation.
    pub fn not_an_optimization() -> Self {
        SyntaxTreeError::NotAnOptimization
    }
    pub fn not_a_symbol_ref() -> Self {
        SyntaxTreeError::NotASymbolRef
    }
}
