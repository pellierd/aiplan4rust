use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::LiftedDurativeAction;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::types::{expr, typed_list};

/// Flattens all union types (`Type::Either`) within a `LiftedDurativeAction` in place.
///
/// This function performs a comprehensive transformation of the durative action by
/// remapping types in its signature, duration, timed conditions, and effects.
///
/// # Parameters
/// - `da`: The `LiftedDurativeAction` to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID`.
///
/// # Returns
/// - `Ok(())` if all components were successfully flattened.
/// - `Err(LirError)` if a type is encountered that is not present in the mapping.
pub fn flatten(
    da: &mut LiftedDurativeAction,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // 1. Flatten action parameters
    // We use the mutable accessor returning &mut TypedList as defined in the LIR.
    typed_list::flatten_typed_variable_list(da.parameters_mut(), map)?;

    // 2. Flatten the duration expression
    // Durative actions include a specific duration expression that may contain
    // functions or variables requiring type remapping.
    expr::flatten(da.duration_mut(), map)?;

    // 3. Flatten timed conditions
    // The `condition_mut()` method provides a mutable reference to the
    // internal expression tree representing preconditions (at start, over all, etc.).
    expr::flatten(da.condition_mut(), map)?;

    // 4. Flatten temporal effects
    // Updates the effect expression tree (at start, at end) to ensure all
    // referenced types match the flattened domain.
    expr::flatten(da.effect_mut(), map)?;

    Ok(())
}
