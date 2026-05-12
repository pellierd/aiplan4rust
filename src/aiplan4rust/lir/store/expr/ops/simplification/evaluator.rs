use crate::aiplan4rust::lir::store::expr::expr::Expr;
use crate::aiplan4rust::lir::store::expr::ops::simplification::StaticValue;

/// Interface permettant au LIR d'interroger des informations extérieures.
pub trait StaticEvaluator: Send + Sync {
    /// Évalue une expression donnée.
    ///
    /// L'objet `Expr` contient déjà l'ID racine et la référence au Store.
    fn evaluate(&self, expr: Expr<'_>) -> Option<StaticValue>;
}
