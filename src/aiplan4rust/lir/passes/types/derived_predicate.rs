use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::types::{atomic_formula_skeleton, expr};

/// Flattens all union types (`Type::Either`) within a `DerivedPredicate` in place.
///
/// This function performs a complete transformation of the derived predicate by:
/// 1. Flattening the **head** (the signature) via `atomic_formula_skeleton`.
/// 2. Flattening the **body** (the logical formula) via `expr`.
///
/// This ensures that both the definition of the derived predicate and its
/// underlying logic use the same canonical primitive types (pivots).
///
/// # Parameters
/// - `derived_predicate`: The derived predicate structure to modify.
/// - `map`: A mapping from union types to their unique flattened primitive type identifiers.
///
/// # Returns
/// - `Ok(())` if the head and the body were successfully flattened.
/// - `Err(LirError)` if any part of the predicate refers to a union type missing from the mapping.
///
/// # Implementation Note
/// Since derived predicates often bridge different parts of the domain, it is
/// critical to types the body to ensure that any variables or sub-expr
/// remain type-consistent with the flattened objects.
pub fn flatten(
    derived_predicate: &mut DerivedPredicate,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // 1. Flatten the signature (head)
    atomic_formula_skeleton::flatten(derived_predicate.head_mut(), map)?;

    // 2. Flatten the logical definition (body)
    expr::flatten(derived_predicate.body_mut(), map)?;

    Ok(())
}
