use thiserror::Error;
use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::NodeId;

#[derive(Error, Debug)]
pub enum DatalogError {
    /// Erreur levée lorsque la pile de l'algorithme de flattening est incohérente.
    /// Cela arrive si un nœud parent attend plus d'enfants que la pile n'en contient.
    #[error("Inconsistent stack state during flattening: Node {0:?} expected more children than available")]
    InconsistentStack(ExprKind),

    /// Erreur levée si, à la fin du parcours, la pile ne contient pas exactement un élément racine.
    #[error("Final stack state is invalid: expected 1 root atom, found {0}")]
    InvalidFinalState(usize),

    /// Erreur levée si un paramètre possède une définition de type invalide ou vide.
    /// Crucial pour la sécurité de l'accès `members()[0]`.
    #[error("Inconsistent type definition for parameter {0:?}: expected a primitive pivot type")]
    InconsistentType(VariableId),

    /// Erreur levée si un identifiant de variable dépasse la capacité du bitset (64).
    #[error("Variable ID {0} exceeds the 64-bit capacity of the flattener")]
    VariableLimitExceeded(u32),

    /// Erreur levée lors de la rencontre d'un nœud non supporté ou inattendu lors du flattening.
    /// Utile pour détecter les quantificateurs non expansés ou les types invalides.
    #[error("Unsupported node type {kind:?} at node {node_id:?}. Ensure expand() was called.")]
    UnsupportedNode {
        kind: ExprKind,
        node_id: NodeId,
    },

    /// Erreur levée lorsqu'un segment attendu (comme les Types ou le Root)
    /// n'a pas été initialisé avant son utilisation.
    #[error("Internal engine state inconsistency: {0}")]
    InternalState(String),

    /// Erreur de passage lors de l'extraction d'atomes ou de la manipulation d'identifiants.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Erreur levée si un argument d'atome n'est ni une variable ni une constante
    #[error("Invalid atom argument at node index {0}")]
    InvalidAtomArgument(NodeId),

    /// Erreur provenant de la couche d'expression LIR.
    #[error(transparent)]
    Expr(#[from] ExprError),
}
