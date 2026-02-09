use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::problem::types::{expr, typed_list};

/// Flattens all union types (`Type::Either`) within an `Action` in place.
///
/// This function performs a complete transformation of the action by:
/// 1. Flattening the **parameters** in the header via `typed_list`.
/// 2. Flattening the **precondition** expression tree via `expr`.
/// 3. Flattening the **effect** expression tree via `expr`.
///
/// # Parameters
/// - `action`: The `Action` structure to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID` (pivots).
///
/// # Returns
/// - `Ok(())` if the header, precondition, and effect were successfully flattened.
/// - `Err(LirError)` if any part of the action refers to a union type missing from the mapping.
///
/// # Implementation Note
/// It is vital to types both preconditions and effects because they often
/// contain quantified variables (forall/exists) or refer to object types that
/// must match the flattened domain.
pub fn flatten(
    action: &mut LiftedAction,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // 1. Flatten parameters in the action header
    // We access the parameters through the header's mutable interface
    typed_list::flatten_typed_variable_list(action.parameters_mut(), map)?;

    // 2. Flatten the precondition expression tree
    expr::flatten(action.precondition_mut(), map)?;

    // 3. Flatten the effect expression tree
    expr::flatten(action.effect_mut(), map)?;

    Ok(())
}
