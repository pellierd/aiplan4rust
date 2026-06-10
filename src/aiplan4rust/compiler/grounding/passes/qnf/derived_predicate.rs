use super::expr;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::ExpansionScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::derived_predicate::DerivedPredicate;

/// Expands all logical quantifiers (`forall` and `exists`) within the predicate's body.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
pub fn expand(
    predicate: &mut DerivedPredicate,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = ExpansionScratchpad::new();
    expand_with(
        predicate,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Expands logical quantifiers within the predicate's body with optional simplification.
///
/// Ce point d'entrée accepte la référence mutable sur le prédicat complet, extrait son `ExprId`
/// interne, et applique la mise à jour directement via son setter.
pub fn expand_with(
    predicate: &mut DerivedPredicate,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut ExpansionScratchpad,
) -> Result<(), GroundingError> {
    // 1. On extrait l'ID actuel du corps par copie (type primitif, aucun conflit d'emprunt)
    let body = predicate.body();

    // 2. On délègue l'expansion à l'algorithme sur le store d'expressions
    let new_body_id = expr::expand_with(
        body,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;

    // 3. On met à jour directement l'instance du prédicat dérivé
    predicate.set_body(new_body_id);

    Ok(())
}
