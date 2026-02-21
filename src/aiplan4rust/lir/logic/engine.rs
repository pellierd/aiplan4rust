use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::lir::logic::evaluator::StaticEvaluator;
use crate::aiplan4rust::lir::logic::rewrite::{
    eliminate_imply, factorize_time_specifier, push_negation, push_time_specifier
};
use crate::aiplan4rust::lir::logic::simplify::simplify;
use crate::aiplan4rust::lir::logic::simplify::simplify::simplify_from;
use crate::aiplan4rust::tree::NodeId;

pub struct LogicEngine<'a> {
    /// L'évaluateur est optionnel : Some pour la réduction (avec inertie),
    /// None pour la normalisation pure.
    evaluator: Option<&'a dyn StaticEvaluator>,
}

impl<'a> LogicEngine<'a> {
    /// Constructeur par défaut : pas d'évaluateur statique.
    pub fn new() -> Self {
        Self { evaluator: None }
    }

    /// Constructeur avec un évaluateur (ex: InertiaRegistry).
    /// On accepte n'importe quoi qui implémente StaticEvaluator.
    pub fn with_evaluator(evaluator: &'a dyn StaticEvaluator) -> Self {
        Self { evaluator: Some(evaluator) }
    }

    pub fn normalize(&self, expr: &mut Expr) -> Result<(), LogicError> {
        let Some(root_id) = expr.root_id() else { return Ok(()); };

        // 1. Mise en forme logique
        eliminate_imply(root_id, expr)?;
        push_negation(root_id, expr)?;

        // 2. Mise en forme temporelle
        if push_time_specifier(root_id, expr)? {
            factorize_time_specifier(root_id, expr)?;
        }

        // 3. Simplification Post-Order (utilise le trait)
        simplify(expr, self.evaluator)?;

        Ok(())
    }

    /// Simplification complète de l'arbre.
    pub fn simplify(&self, expr: &mut Expr) -> Result<(), LogicError> {
        simplify(expr, self.evaluator)
    }

    /// Simplification ciblée à partir d'un nœud (très efficace après expansion).
    pub fn simplify_from(&self, expr: &mut Expr, node_id: NodeId) -> Result<(), LogicError> {
        simplify_from(expr, node_id, self.evaluator)
    }
}
