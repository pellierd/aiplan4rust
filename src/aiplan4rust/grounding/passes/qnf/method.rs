use crate::aiplan4rust::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::qnf::ExpansionScratchpad;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::expr::ExprStore;
use crate::aiplan4rust::lir::problem::MethodDef;
// Ajuste si MethodDef est dans htn ou problem

use super::expr;

/// Expands all logical quantifiers (`forall` and `exists`) within a Method's expressions.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
pub fn expand(
    method: &mut MethodDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = ExpansionScratchpad::new();
    expand_with(
        method,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Expands logical quantifiers within a Method, with optional on-the-fly simplification.
///
/// Ce point d'entrée accepte les scratchpads de binding et d'expansion pré-alloués
/// pour garantir l'absence totale d'allocation à chaud lors du traitement des méthodes HTN.
pub fn expand_with(
    method: &mut MethodDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut ExpansionScratchpad,
) -> Result<(), GroundingError> {
    // 1. Expand Preconditions
    // Note: Si .precondition() renvoie un objet `Expr`, remplace par `method.precondition().root_id()`
    let current_precondition_id = method.precondition();
    let new_precondition_id = expr::expand_with(
        current_precondition_id,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    method.set_precondition(new_precondition_id);

    // 2. Expand Task Network Constraints
    // Note: Si .logical_constraints() renvoie un objet `Expr`, remplace par `.logical_constraints().root_id()`
    let current_constraints_id = method.task_network().logical_constraints();
    let new_constraints_id = expr::expand_with(
        current_constraints_id,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    method
        .task_network_mut()
        .set_logical_constraints(new_constraints_id); // Ajuste selon le nom exact de ton setter de contraintes

    Ok(())
}
