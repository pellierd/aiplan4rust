use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{FunctorID, LangError, PredicateID, StringID, Type};
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use thiserror::Error;
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::tree::NodeId;

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
    TypeNotFound(StringID),

    /// Constant not found for a given Ident.
    #[error("Constant with id {0:?} not found")]
    ConstantNotFound(StringID),

    /// Object not found for a given Ident.
    #[error("Object with id {0:?} not found")]
    ObjectNotFound(StringID),

    /// Missing type when remap types
    #[error("Missing type in flattened hierarchy: {ty:?}")]
    MissingType { ty: Type<StringID> },

    #[error("Inertia missing for predicate: {id:?}")]
    MissingPredicateInertia { id: PredicateID },

    /// L'inertie de la fonction est introuvable dans la table.
    #[error("Inertia missing for function: {id:?}")]
    MissingFunctionInertia { id: FunctorID },

    #[error("Failed to bind {symbol}")]
    SymbolBindingFailed { symbol: Symbol },

    #[error("Failed to bind type: {ty:?})")]
    TypeBindingFailed { ty: Type<StringID> },

    #[error("Failed to find variable with node id: {node_id:?})")]
    VariableNotFound { node_id: NodeId },

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
    pub fn type_not_found(id: StringID) -> Self {
        LirError::TypeNotFound(id)
    }

    /// Creates a `ConstantNotFound` error for the given `Ident`.
    pub fn constant_not_found(id: StringID) -> Self {
        LirError::ConstantNotFound(id)
    }

    /// Creates an `ObjectNotFound` error for the given `Ident`.
    pub fn object_not_found(id: StringID) -> Self {
        LirError::ObjectNotFound(id)
    }

    /// Creates a new `MissingType` error for the given type.
    pub fn missing_type(ty: Type<StringID>) -> Self {
        LirError::MissingType { ty }
    }

    /// Creates a new error indicating that inertia information is missing for a predicate.
    ///
    /// This error occurs when a predicate is encountered during the encoding or
    /// analysis phase but has no corresponding entry in the inertia table,
    /// suggesting it was skipped during the initial state or effect scanning pass.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the missing predicate.
    pub fn missing_predicate_inertia(id: PredicateID) -> Self {
        Self::MissingPredicateInertia { id }
    }

    /// Creates a new error indicating that inertia information is missing for a function.
    ///
    /// This error occurs when a numeric function is encountered but lacks
    /// an entry in the inertia table, preventing the system from determining
    /// if it is a constant or a fluent.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the missing function.
    pub fn missing_function_inertia(id: FunctorID) -> Self {
        Self::MissingFunctionInertia { id }
    }

    /// Creates a new binding error.
    ///
    /// # Parameters
    /// - `kind`: The kind of symbol (from your semantic analysis).
    /// - `node_id`: The ID of the AST node.
    pub fn symbol_binding_failed(symbol: Symbol) -> Self {
        Self::SymbolBindingFailed { symbol }
    }

    pub fn type_binding_failed(ty: Type<StringID>) -> Self {
        Self::TypeBindingFailed { ty }
    }

    pub fn variable_not_found(node_id: NodeId) -> Self {
        Self::VariableNotFound { node_id }
    }
}
