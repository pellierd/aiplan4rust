use crate::aiplan4rust::lir::analysis::inertia::registry::InertiaRegistry;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::lir::logic::rewrite::{eliminate_imply, factorize_time_specifier, push_negation, push_time_specifier};
use crate::aiplan4rust::lir::logic::simplify::simplify;

pub struct LogicEngine<'a> {
    // L'index est optionnel : Some pour la réduction, None pour la normalisation pure
    inertia_registry: Option<&'a InertiaRegistry<'a>>,
}

impl<'a> LogicEngine<'a> {
    /// Constructeur sans argument : index = None
    pub fn new() -> Self {
        Self { inertia_registry: None }
    }

    /// Constructeur avec argument
    pub fn from(index: &'a InertiaRegistry) -> Self {
        Self { inertia_registry: Some(index) }
    }

    /// On change la forme de l'expression (NNF, Elimination des Imply, etc.)
    pub fn normalize(&self, expr: &mut Expr) -> Result<(), LogicError> {
            let Some(root_id) = expr.root_id() else { return Ok(()); };

            eliminate_imply(root_id, expr)?;
            push_negation(root_id, expr)?;

            if push_time_specifier(root_id, expr)? {
                factorize_time_specifier(root_id, expr)?;
            }

            simplify(root_id, expr, None)?;
            // TO ADD as post-processing afet simplify
            // Factorization
            // Example: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
            Ok(())
    }


    /// On réduit les constantes et on utilise l'index statique si présent.
    pub fn reduce(&self, expr: &mut Expr) -> Result<(), LogicError> {
        // On appelle le point d'entrée de ton module simplify
        // qui va lui-même dispatcher vers and_or.rs, arithmetic.rs, etc.
        //simplify::run(expr, self.index)
        Ok(())
    }
}
