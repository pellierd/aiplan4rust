use crate::aiplan4rust::lir::expr::ops;
use crate::aiplan4rust::lir::LiftedDerivedPredicate;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;

/// Normalizes the logical expression of a `DerivedPredicate` using the provided `LogicEngine`.
///
/// This function applies expression expr only to the `body` of the
/// derived predicate. The `head` (predicate name and parameters) is not modified.
///
/// # Arguments
///
/// * `substitution` - The ops substitution to use for expr.
/// * `derived_predicate` - A mutable reference to the `DerivedPredicate` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the `body` expression fails.
pub fn normalize(
    derived_predicate: &mut LiftedDerivedPredicate
) -> Result<(), ExprOpError> {
    ops::normalize(derived_predicate.body_mut())
}
