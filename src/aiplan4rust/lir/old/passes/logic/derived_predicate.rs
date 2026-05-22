use crate::aiplan4rust::lir::old::expr::ops;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::DerivedPredicateDef;

/// Normalizes the logical expression of a `DerivedPredicate` using the provided `LogicEngine`.
///
/// This function applies expression logic only to the `body` of the
/// derived predicate. The `head` (predicate name and parameters) is not modified.
///
/// # Arguments
///
/// * `binding` - The ops binding to use for logic.
/// * `derived_predicate` - A mutable reference to the `DerivedPredicate` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if logic of the `body` expression fails.
pub fn normalize(derived_predicate: &mut DerivedPredicateDef) -> Result<(), ExprOpError> {
    ops::normalize(derived_predicate.body_mut())
}
