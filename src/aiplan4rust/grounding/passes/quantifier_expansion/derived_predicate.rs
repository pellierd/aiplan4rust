use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::quantifier_expansion::expr;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::store::expr_old::ops::StaticEvaluator;
use crate::aiplan4rust::lir::DerivedPredicateDef;

/// Expands all logical quantifiers (`forall` and `exists`) within the predicate's body.
///
/// This is a convenience wrapper around [`expand_with`] that performs expansion
/// without any additional static simplification.
pub fn expand(
    predicate: &mut DerivedPredicateDef,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    expand_with(predicate, value_registry, None)
}

/// Expands logical quantifiers within the predicate's body with optional simplification.
///
/// This function focuses strictly on the `body` of the `DerivedPredicate`. It
/// replaces quantified variables with concrete object instances from the
/// `ValueRegistry`.
///
/// # Arguments
/// * `predicate` - A mutable reference to the derived predicate to transform.
/// * `value_registry` - The evaluator containing object constants for substitution.
/// * `evaluator` - Optional static evaluator to prune the expression tree during expansion.
pub fn expand_with(
    predicate: &mut DerivedPredicateDef,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), GroundingError> {
    // Expand quantifiers (forall/exists) inside the definition (body).
    // The head_id and head remain untouched as they define the signature.
    expr::expand_with(&mut predicate.body_mut(), value_registry, evaluator)?;

    Ok(())
}
