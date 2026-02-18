use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId, LangError, ObjectId, SymbolId, TaskSkeletonId, Type, TypeId};
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::lir::symbol_registry::IndexTableError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::tree::NodeId;

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


    /// An error originating from the expression system.
    #[error(transparent)]
    Logic(#[from] LogicError),

    /// An error originating from the expression system.
    #[error(transparent)]
    Inertia(#[from] InertiaError),
    
    #[error(transparent)]
    IndexTable(#[from] IndexTableError),

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
    TypeNotFound(SymbolId),

    /// Constant not found for a given Ident.
    #[error("Constant with id {0:?} not found")]
    ConstantNotFound(SymbolId),

    /// Object not found for a given Ident.
    #[error("Object with id {0:?} not found")]
    ObjectNotFound(SymbolId),

    /// Missing type when remap types
    #[error("Missing type in flattened hierarchy: {ty:?}")] // Changed {types:?} to {ty:?}
    MissingType { ty: Type<TypeId> },


    #[error("Failed to bind {symbol}")]
    SymbolBindingFailed { symbol: NodeId },

    #[error("Failed to bind type: {ty:?}")] // Changed {types:?} to {ty:?}
    TypeBindingFailed { ty: Type<SymbolId> },

    #[error("Failed to find variable with node id: {node_id:?})")]
    VariableNotFound { node_id: NodeId },

    #[error("Index out of bound: {id})")]
    IndexOutOfBound { id: usize },

    // A definition was provided for a type that was never registered in the symbol table.
    #[error("Type definition provided for an unregistered ID: {id:?}")]
    TypeDefinitionOrphan { id: TypeId },

    // In the LirError enum
    #[error("Object definition provided for an unregistered ID: {id:?}")]
    ObjectDefinitionOrphan { id: ObjectId },

    #[error("Predicate definition requested for an unregistered ID: {id:?}")]
    PredicateDefinitionOrphan { id: AtomSkeletonId },

    #[error("Function definition requested for an unregistered ID: {id:?}")]
    FunctionDefinitionOrphan { id: FunctionSkeletonId },

    // In your LirError enum
    #[error("Task definition requested for an unregistered ID: {id:?}")]
    TaskDefinitionOrphan { id: TaskSkeletonId },

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
    pub fn type_not_found(id: SymbolId) -> Self {
        LirError::TypeNotFound(id)
    }

    /// Creates a `ConstantNotFound` error for the given `Ident`.
    pub fn constant_not_found(id: SymbolId) -> Self {
        LirError::ConstantNotFound(id)
    }

    /// Creates an `ObjectNotFound` error for the given `Ident`.
    pub fn object_not_found(id: SymbolId) -> Self {
        LirError::ObjectNotFound(id)
    }

    /// Creates a new `MissingType` error for the given type.
    pub fn missing_type(ty: Type<TypeId>) -> Self {
        LirError::MissingType { ty }
    }



    /// Creates a new binding error.
    ///
    /// # Parameters
    /// - `kind`: The kind of symbol (from your semantic analysis).
    /// - `node_id`: The ID of the AST node.
    #[track_caller]
    pub fn symbol_binding_failed(symbol: NodeId) -> Self {
        let err = Self::SymbolBindingFailed { symbol };
        Self::log_error(&err, std::panic::Location::caller());
        err
    }

    #[track_caller]
    pub fn type_binding_failed(ty: Type<SymbolId>) -> Self {
        let err = Self::TypeBindingFailed { ty };
        Self::log_error(&err, std::panic::Location::caller());
        err
    }

    #[track_caller]
    pub fn variable_not_found(node_id: NodeId) -> Self {
        let err = Self::VariableNotFound { node_id };
        Self::log_error(&err, std::panic::Location::caller());
        err
    }

    pub fn index_out_of_bounds(id: usize) -> Self {
        Self::IndexOutOfBound { id }
    }

    pub fn type_definition_orphan(id: TypeId) -> Self {
        Self::TypeDefinitionOrphan { id }
    }

    // In the LirError impl block
    pub fn object_definition_orphan(id: ObjectId) -> Self {
        Self::ObjectDefinitionOrphan { id }
    }

    pub fn predicate_definition_orphan(id: AtomSkeletonId) -> Self {
        Self::PredicateDefinitionOrphan { id }
    }

    /// Constructeur pour l'erreur de fonction
    pub fn function_definition_orphan(id: FunctionSkeletonId) -> Self {
        Self::FunctionDefinitionOrphan { id }
    }

    // In your impl LirError block
    pub fn task_definition_orphan(id: TaskSkeletonId) -> Self {
        Self::TaskDefinitionOrphan { id }
    }




    /// Helper privé pour uniformiser le logging et la stack trace sans polluer les fonctions publiques
    fn log_error(err: &Self, caller: &std::panic::Location) {
        if log::log_enabled!(log::Level::Debug) {
            let bt = std::backtrace::Backtrace::force_capture();
            log::debug!(
            "\nLIR Error at {}:{}:{}\n{}\nStack trace:\n{}",
            caller.file(),
            caller.line(),
            caller.column(),
            err,
            bt
        );
        }
    }
}
