use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::interner::{Ident, InternerError};
use crate::aiplan4rust::lang::{LangError, Type};
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use thiserror::Error;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

/// Represents errors that can occur within the `lir` (Lifted Intermediate Representation) module.
///
/// This enum aggregates possible error types arising during the construction,
/// manipulation, or conversion of the intermediate representation in the compiler or analysis pipeline.
/// It encapsulates errors from several subsystems, as well as module-specific errors.
///
/// # Variants
///
/// - [`Expr`]: Errors originating from the expression subsystem.
/// - [`Lang`]: Errors originating from the language module.
/// - [`Ast`]: Errors originating from AST (Abstract Syntax Tree) processing.
/// - [`SyntaxTree`]: Errors from the underlying syntax tree system.
/// - [`Arena`]: Errors related to arena allocation and memory management.
/// - [`ActionAstKindError`]: Error when an unexpected `AstKind` is encountered in an Action conversion.
/// - [`UnsupportedTaskNetwork`]: Error when a task network's AST structure is not supported.
/// - [`InternalError`]: Generic internal errors indicating unexpected or unrecoverable conditions.
#[derive(Debug, Error)]
pub enum LirError {

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// An error originating from the expression system.
    #[error(transparent)]
    Expr(#[from] ExprError),

    /// An error originating from the language module.
    #[error(transparent)]
    Lang(#[from] LangError),

    /// An error originating from the AST subsystem.
    #[error(transparent)]
    Ast(#[from] AstError),

    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error related to arena allocation.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error originating from the string interner.
    #[error(transparent)]
    Interner(#[from] InternerError),

    /// Indicates an unexpected `AstKind` was encountered during Action conversion.
    #[error("Unexpected AstKind in Action conversion: {0:?}")]
    ActionAstKindError(AstKind),

    /// Indicates an unsupported or invalid task network AST kind encountered during conversion.
    #[error("Unexpected AstKind in TaskNetwork conversion: {0:?}")]
    TaskNetworkAstKindError(AstKind),

    /// Type not found for a given Ident.
    #[error("Type with id {0:?} not found")]
    TypeNotFound(Ident),

    /// Constant not found for a given Ident.
    #[error("Constant with id {0:?} not found")]
    ConstantNotFound(Ident),

    /// Object not found for a given Ident.
    #[error("Object with id {0:?} not found")]
    ObjectNotFound(Ident),

    /// Missing type when remap types
    #[error("Missing type in flattened hierarchy: {ty:?}")]
    MissingType { ty: Type },

    #[error("Inertia information missing for predicate: {id:?}")]
    InertiaInformationMissingPredicate { id: usize },

    #[error("Inertia information missing for function: {id:?}")]
    InertiaInformationMissingFunction { id: usize },
}

impl LirError {
    /// Constructs a `LirError` from an [`ExprError`].
    pub fn expr(err: ExprError) -> Self {
        LirError::Expr(err)
    }

    /// Constructs a `LirError` from a [`SyntaxTreeError`].
    pub fn syntax_tree(err: SyntaxTreeError) -> Self {
        LirError::SyntaxTree(err)
    }

    /// Creates an [`ActionAstKindError`] error from the unexpected [`AstKind`].
    pub fn action_ast_kind_error(kind: AstKind) -> Self {
        LirError::ActionAstKindError(kind)
    }

    /// Creates an [`UnsupportedTaskNetwork`] error with a custom message.
    ///
    /// # Arguments
    ///
    /// * `msg` - A string describing the unsupported task network structure.
    /// Creates a `TaskNetworkAstKindError` from the unexpected `AstKind`.
    pub fn task_network_ast_kind_error(kind: AstKind) -> Self {
        LirError::TaskNetworkAstKindError(kind)
    }

    /// Creates a `TypeNotFound` error for the given `Ident`.
    pub fn type_not_found(id: Ident) -> Self {
        LirError::TypeNotFound(id)
    }

    /// Creates a `ConstantNotFound` error for the given `Ident`.
    pub fn constant_not_found(id: Ident) -> Self {
        LirError::ConstantNotFound(id)
    }

    /// Creates an `ObjectNotFound` error for the given `Ident`.
    pub fn object_not_found(id: Ident) -> Self {
        LirError::ObjectNotFound(id)
    }

    /// Creates a new `MissingType` error for the given type.
    pub fn missing_type(ty: Type) -> Self {
        LirError::MissingType { ty }
    }

    /// Creates a new [`LirError::InertiaInformationMissingPredicate`] error.
    ///
    /// This error should be raised when a predicate is encountered during
    /// expansion or grounding but has no entry in the [`InertiaTable`].
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier (index) of the predicate that was
    ///   not processed during the inertia analysis pass.
    ///
    /// # Returns
    ///
    /// Returns a variant of [`LirError`] containing the missing predicate index.
    pub fn inertia_information_missing_predicate(id: usize) -> Self {
        LirError::InertiaInformationMissingPredicate { id }
    }

    /// Creates a new [`LirError::InertiaInformationMissingFunction`] error.
    ///
    /// This error should be raised when a function (numeric fluent) is encountered
    /// during expansion or grounding but has no entry in the [`InertiaTable`].
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier (index) of the function that was
    ///   not processed during the inertia analysis pass.
    ///
    /// # Returns
    ///
    /// Returns a variant of [`LirError`] containing the missing function index.
    pub fn inertia_information_missing_function(id: usize) -> Self {
        LirError::InertiaInformationMissingFunction { id }
    }
}
