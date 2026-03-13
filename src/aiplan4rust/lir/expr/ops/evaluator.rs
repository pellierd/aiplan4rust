use ordered_float::OrderedFloat;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::lang::ObjectId;
use crate::aiplan4rust::lir::expr::Expr;

/// Représente une valeur constante extraite par une analyse statique (ex: Inertie).
/// Ce either_type sert de langage commun entre le LIR et les modules externes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StaticValue {
    /// Valeur booléenne (généralement pour un prédicat statique)
    Boolean(bool),
    /// Valeur numérique (pour une fonction inerte)
    Number(OrderedFloat<f64>),
    /// Référence à un objet constant (pour une fonction inerte retournant un objet)
    Object(ObjectId),
}

/// Interface permettant à la logique du LIR d'interroger des informations
/// extérieures (comme l'inertie) sans dépendre de l'implémentation du Grounding.
pub trait StaticEvaluator : Send + Sync {
    /// Évalue un nœud de l'expression (AtomicFormula ou FunctionTerm).
    ///
    /// Retourne `Some(StaticValue)` si l'analyse confirme que le nœud est statique/constant,
    /// ou `None` si la valeur est inconnue ou dynamique.
    fn evaluate(&self, node_id: NodeId, expr: &Expr) -> Option<StaticValue>;
}
