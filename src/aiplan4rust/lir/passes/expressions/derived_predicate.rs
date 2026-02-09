use crate::aiplan4rust::lir::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::{expr, LiftedDerivedPredicate};
use crate::aiplan4rust::lir::expr::ExprError;

/// Normalizes the logical expression of a `DerivedPredicate`.
///
/// This function applies expression normalization only to the `body` of the
/// derived predicate. The `head` (predicate name and parameters) is not modified.
///
/// This function is intended for internal use within the `problem` module
/// and is not part of the public API.
///
/// # Arguments
///
/// * `derived_predicate` - A mutable reference to the `DerivedPredicate` to normalize.
///
/// # Errors
///
/// Returns an [`ExprError`] if normalization of the `body` expression fails.
///
/// # Example
///
/// ```rust,ignore
/// # use aiplan4rust::lir::DerivedPredicate;
/// # use aiplan4rust::lir::expr::ExprError;
/// # fn example(derived: &mut DerivedPredicate) -> Result<(), ExprError> {
/// normalize(derived)?;
/// # Ok(())
/// # }
/// ```
pub fn normalize(
    derived_predicate: &mut LiftedDerivedPredicate
) -> Result<(), ExprError> {
    expr::normalize(derived_predicate.body_mut())
}
