use crate::aiplan4rust::lir::LiftedDerivedPredicate;
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};

/// Normalizes the logical expression of a `DerivedPredicate` using the provided `LogicEngine`.
///
/// This function applies expression expr only to the `body` of the
/// derived predicate. The `head` (predicate name and parameters) is not modified.
///
/// # Arguments
///
/// * `engine` - The logic engine to use for expr.
/// * `derived_predicate` - A mutable reference to the `DerivedPredicate` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the `body` expression fails.
pub fn normalize(
    engine: &LogicEngine,
    derived_predicate: &mut LiftedDerivedPredicate
) -> Result<(), LogicError> {
    engine.normalize(derived_predicate.body_mut())
}
