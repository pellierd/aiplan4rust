use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{
    AtomSkeletonId, FunctionSkeletonId, LangError, ObjectId, PreferenceSymbolId, SymbolId,
    TaskSkeletonId, Type, TypeId,
};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::problem::symbol_registry::IndexTableError;
use crate::aiplan4rust::lir::store::encoding::EncodingError;
use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use thiserror::Error;

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
    ExprOpHC(#[from] ExprOpErrorHC),

    /// An error originating from the expression system.
    #[error(transparent)]
    Logic(#[from] ExprOpError),

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

    /// Missing typing when remap types
    #[error("Missing typing in flattened hierarchy: {ty:?}")] // Changed {types:?} to {ty:?}
    MissingType { ty: Type<TypeId> },

    #[error("Failed to bind {symbol}")]
    SymbolBindingFailed { symbol: NodeId },

    #[error("Failed to bind typing: {ty:?}")] // Changed {types:?} to {ty:?}
    TypeBindingFailed { ty: Type<SymbolId> },

    #[error("Failed to find variable with node id: {node_id:?})")]
    VariableNotFound { node_id: NodeId },

    #[error("Index out of bound: {id})")]
    IndexOutOfBound { id: usize },

    // A definition was provided for a typing that was never registered in the symbol table.
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

    // Dans l'enum LirError
    #[error("Preference definition requested for an unregistered ID: {id:?}")]
    PreferenceDefinitionOrphan { id: PreferenceSymbolId },

    #[error(transparent)]
    Encoding(#[from] EncodingError),
}

impl LirError {
    /// Constructs a `LirError` from an [`ExprError`].
    #[track_caller]
    pub fn expr(err: ExprError) -> Self {
        LirError::Expr(err).trace()
    }

    /// Constructs a `LirError` from a [`SyntaxTreeError`].
    #[track_caller]
    pub fn syntax_tree(err: SyntaxTreeError) -> Self {
        LirError::SyntaxTree(err).trace()
    }

    /// Creates an [`ActionAstKindError`] error from the unexpected [`AstKind`].
    #[track_caller]
    pub fn action_ast_kind_error(kind: AstKind) -> Self {
        LirError::ActionAstKindError(kind).trace()
    }

    /// Creates an [`UnsupportedTaskNetwork`] error with a custom message.
    ///
    /// # Arguments
    ///
    /// * `msg` - A string describing the unsupported task network structure.
    /// Creates a `TaskNetworkAstKindError` from the unexpected `AstKind`.
    #[track_caller]
    pub fn task_network_ast_kind_error(kind: AstKind) -> Self {
        LirError::TaskNetworkAstKindError(kind).trace()
    }

    /// Creates a `TypeNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn type_not_found(id: SymbolId) -> Self {
        LirError::TypeNotFound(id).trace()
    }

    /// Creates a `ConstantNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn constant_not_found(id: SymbolId) -> Self {
        LirError::ConstantNotFound(id).trace()
    }

    /// Creates an `ObjectNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn object_not_found(id: SymbolId) -> Self {
        LirError::ObjectNotFound(id).trace()
    }

    /// Creates a new `MissingType` error for the given typing.
    #[track_caller]
    pub fn missing_type(ty: Type<TypeId>) -> Self {
        LirError::MissingType { ty }.trace()
    }

    /// Creates a new binding error.
    ///
    /// # Parameters
    /// - `kind`: The kind of symbol (from your semantic analysis).
    /// - `node_id`: The ID of the AST node.
    #[track_caller]
    pub fn symbol_binding_failed(symbol: NodeId) -> Self {
        Self::SymbolBindingFailed { symbol }.trace()
    }

    #[track_caller]
    pub fn type_binding_failed(ty: Type<SymbolId>) -> Self {
        Self::TypeBindingFailed { ty }.trace()
    }

    #[track_caller]
    pub fn variable_not_found(node_id: NodeId) -> Self {
        Self::VariableNotFound { node_id }.trace()
    }

    #[track_caller]
    pub fn index_out_of_bounds(id: usize) -> Self {
        Self::IndexOutOfBound { id }.trace()
    }

    #[track_caller]
    pub fn type_definition_orphan(id: TypeId) -> Self {
        Self::TypeDefinitionOrphan { id }.trace()
    }

    // In the LirError impl block
    #[track_caller]
    pub fn object_definition_orphan(id: ObjectId) -> Self {
        Self::ObjectDefinitionOrphan { id }.trace()
    }

    #[track_caller]
    pub fn predicate_definition_orphan(id: AtomSkeletonId) -> Self {
        Self::PredicateDefinitionOrphan { id }
    }

    /// Constructeur pour l'erreur de fonction

    #[track_caller]
    pub fn function_definition_orphan(id: FunctionSkeletonId) -> Self {
        Self::FunctionDefinitionOrphan { id }.trace()
    }

    // In your impl LirError block
    #[track_caller]
    pub fn task_definition_orphan(id: TaskSkeletonId) -> Self {
        Self::TaskDefinitionOrphan { id }.trace()
    }

    #[track_caller]
    pub fn preference_definition_orphan(id: PreferenceSymbolId) -> Self {
        Self::PreferenceDefinitionOrphan { id }.trace()
    }
}

impl Traceable for LirError {}
