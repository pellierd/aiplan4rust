use crate::aiplan4rust::lir::store::expr::ops::rewriting::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ExprStore;
use crate::aiplan4rust::lir::store::normalization::error::NormalizationError;
use crate::aiplan4rust::lir::store::normalization::logic::expr;
use crate::aiplan4rust::lir::store::problem::DerivedPredicateDef;

/// Normalizes the logical expression of a `DerivedPredicate`.
///
/// This function applies the canonical normalization pipeline directly to the `body`
/// (the condition) of the derived predicate via `expr::normalize`. The `head`
/// (predicate symbol and parameters definition) remains structural and unchanged.
///
/// # Arguments
///
/// * `derived_predicate` - A mutable reference to the `DerivedPredicateDef` to normalize.
/// * `store` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if the normalization or simplification of the
/// body expression fails.
pub fn normalize(
    derived_predicate: &mut DerivedPredicateDef,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // Derived predicates represent static logical rules, so durative logic is false.
    let normalized_body = expr::normalize(derived_predicate.body(), store, scratch, false)?;

    derived_predicate.set_body(normalized_body);

    Ok(())
}
