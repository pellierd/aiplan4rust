use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTableError;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, VariableId};
use crate::analysis::reachability::datalog::core::Term;
use thiserror::Error;

/// Errors encountered during the Datalog grounding and flattening process.
///
/// This enum covers inconsistencies in the internal state of the Datalog engine,
/// unsupported PDDL/LIR constructs, and capacity limits.
#[derive(Error, Debug)]
pub enum DatalogError {
    /// Modifié : Plus besoin de String, le message utilisateur est fixe.
    #[error("Problem not loaded: The grounding engine requires a problem to be loaded via `load_problem` before execution.")]
    ProblemNotLoaded,

    /// Ajout : Le prédicat référencé par son ID n'existe pas dans les définitions.
    #[error("Undefined predicate with ID: {0:?}")]
    UndefinedPredicate(AtomSkeletonId), // Ou l'ID spécifique utilisé (ex: PredicateId)

    /// The construct is valid PDDL/ADL, but the current Datalog encoder
    /// has not implemented it yet.
    ///
    /// This typically applies to complex negations, implications, or quantifiers
    /// that require an explicit Preprocessing step (like Positive Normal Form
    /// or Quantifier Expansion).
    #[error(
        "Feature not supported: {feature} at node {node_id:?}. \
     This ADL construct requires additional preprocessing or a more advanced encoder."
    )]
    FeatureNotSupported { feature: String, node_id: ExprId },

    /// The node encountered is fundamentally incompatible with Datalog grounding.
    ///
    /// This happens when the expression tree contains elements that cannot be
    /// mapped to Horn logic, such as HTN tasks, preferences, or internal
    /// compiler artifacts.
    #[error(
        "Incompatible node: {kind:?} at node {node_id:?}. \
     This element cannot be grounded into Datalog rules."
    )]
    IncompatibleNode { kind: ExprKind, node_id: ExprId },

    /// Raised when a variable in the rule head is not bound by any atom in the body.
    #[error("Unbound variable '{0:?}' in rule head. All variables in the head must appear in the positive body.")]
    UnboundVariable(VariableId),

    /// Raised if an atom argument is neither a variable nor a constant.
    #[error("Invalid atom argument at node index {0}")]
    InvalidAtomArgument(ExprId),

    #[error("Variable limit exceeded: Variable index '{var_id:?}' exceeds the maximum allowed variables per scope ({limit}).")]
    VariableLimitExceeded { var_id: VariableId, limit: usize },

    /// Raised when a required internal segment (e.g., Type registry or Root node)
    /// has not been initialized before use.
    #[error("Internal engine state inconsistency: {0}")]
    InternalState(String),

    /// Errors propagated from the underlying syntax tree manipulation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    #[error(transparent)]
    Store(#[from] StorerError),
}

impl Traceable for DatalogError {}

impl DatalogError {
    /// Crée une erreur `ProblemNotLoaded` et capture la trace sans aucun paramètre.
    #[track_caller]
    pub fn problem_not_loaded() -> Self {
        DatalogError::ProblemNotLoaded.trace()
    }

    /// Crée une erreur UndefinedPredicate et capture la trace.
    #[track_caller]
    pub fn undefined_predicate(id: AtomSkeletonId) -> Self {
        DatalogError::UndefinedPredicate(id).trace()
    }

    /// Creates a `FeatureNotSupported` error, typically for missing ADL transformations.
    #[track_caller]
    pub fn feature_not_supported<S: Into<String>>(feature: S, node_id: ExprId) -> Self {
        DatalogError::FeatureNotSupported {
            feature: feature.into(),
            node_id,
        }
        .trace()
    }

    /// Creates an `IncompatibleExpr` error for nodes that don't belong in a Datalog pipeline.
    #[track_caller]
    pub fn incompatible_node(kind: ExprKind, node_id: ExprId) -> Self {
        DatalogError::IncompatibleNode { kind, node_id }.trace()
    }

    /// Creates an `InternalState` error with a custom message and captures the trace.
    #[track_caller]
    pub fn internal_state<S: Into<String>>(msg: S) -> Self {
        DatalogError::InternalState(msg.into()).trace()
    }

    /// Creates an `InvalidAtomArgument` error for the specified node and captures the trace.
    #[track_caller]
    pub fn invalid_atom_argument(node_id: ExprId) -> Self {
        DatalogError::InvalidAtomArgument(node_id).trace()
    }

    // Ajoute ce helper pour la traçabilité
    #[track_caller]
    pub fn unbound_variable(v: VariableId) -> Self {
        DatalogError::UnboundVariable(v).trace()
    }

    /// 🛡️ AJOUT : Constructeur traçable pour le dépassement de la capacité du bitmask
    #[track_caller]
    pub fn variable_limit_exceeded(var_id: VariableId, limit: usize) -> Self {
        DatalogError::VariableLimitExceeded { var_id, limit }.trace()
    }
}
