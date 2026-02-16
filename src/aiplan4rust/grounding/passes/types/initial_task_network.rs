use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::types::{typed_list};

/// Flattens all union types (`Type::Either`) within an `InitialTaskNetwork` in place.
///
/// This remaps the types of the initial task network parameters to match
/// the primitive types of the flattened domain.
///
/// # Parameters
/// - `itn`: The `InitialTaskNetwork` structure to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID`.
///
/// # Returns
/// - `Ok(())` if the parameters were successfully flattened.
/// - `Err(LirError)` if a union type is missing from the mapping.
pub fn flatten(
    itn: &mut InitialTaskNetwork,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // 1. Flatten the Initial Task Network parameters
    // This ensures that variables declared in the problem header are correctly typed.
    typed_list::flatten_typed_variable_list(itn.parameters_mut(), map)?;

    // Note: If your HDDL implementation requires updating task calls within the
    // task_network in the future, you would add an expr::types call here.

    Ok(())
}
