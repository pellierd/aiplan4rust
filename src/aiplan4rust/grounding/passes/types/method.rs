use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::LiftedMethod;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::types::{expr, typed_list};

/// Flattens all union types (`Type::Either`) within a `Method` in place.
///
/// This transformation ensures that the method's signature, the task it refines,
/// its applicability preconditions, and its decomposition network all use
/// primitive types from the flattened domain.
///
/// # Parameters
/// - `method`: The `Method` structure to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID`.
///
/// # Returns
/// - `Ok(())` if all components were successfully flattened.
/// - `Err(LirError)` if a union type is encountered that is not in the mapping.
pub fn flatten(
    method: &mut LiftedMethod,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    // 1. Flatten method parameters
    // Remaps types in the method's signature (NamedTypedList).
    typed_list::flatten_typed_variable_list(method.parameters_mut(), map)?;

    // 2. Flatten the task expression
    // Remaps types for the arguments of the abstract task being refined.
    expr::flatten(method.task_mut(), map)?;

    // 3. Flatten the precondition expression tree
    // Remaps types used in the logical conditions of the method.
    expr::flatten(method.precondition_mut(), map)?;

    Ok(())
}
