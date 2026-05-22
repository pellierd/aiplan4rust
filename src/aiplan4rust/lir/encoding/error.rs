use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{SymbolId, Type, TypeId};
use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::problem::LiftedProblemError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::{AstError, AstKind};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodingError {
    // Utilisation des accolades pour nommer l'attribut explicitement
    #[error("Unsupported AST node kind: {kind:?}")]
    UnsupportedAstNodeKind { kind: AstKind },

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

    #[error("Builder error: {0}")]
    Builder(#[from] ExprBuilderError),

    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error related to arena allocation.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error originating from the AST subsystem.
    #[error(transparent)]
    Ast(#[from] AstError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    #[error(transparent)]
    LiftedProblem(#[from] LiftedProblemError),

    #[error(transparent)]
    Interner(#[from] InternerError),
}

impl EncodingError {
    /// Constructeur pratique pour l'erreur de nœud non supporté
    pub fn unsupported_ast_node_kind(kind: AstKind) -> Self {
        // On instancie avec le nom de l'attribut
        Self::UnsupportedAstNodeKind { kind }.trace()
    }

    /// Creates a `TypeNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn type_not_found(id: SymbolId) -> Self {
        EncodingError::TypeNotFound(id).trace()
    }

    /// Creates a `ConstantNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn constant_not_found(id: SymbolId) -> Self {
        EncodingError::ConstantNotFound(id).trace()
    }

    /// Creates an `ObjectNotFound` error for the given `Ident`.
    #[track_caller]
    pub fn object_not_found(id: SymbolId) -> Self {
        EncodingError::ObjectNotFound(id).trace()
    }

    /// Creates a new `MissingType` error for the given typing.
    #[track_caller]
    pub fn missing_type(ty: Type<TypeId>) -> Self {
        EncodingError::MissingType { ty }.trace()
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
}

// Implémentation des traits de tracking d'erreurs
impl Traceable for EncodingError {}
