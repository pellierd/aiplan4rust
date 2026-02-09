use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::types::{ty, typed_list};

/// Flattens all types within an `AtomicFunctionSkeleton` in place according to the provided mapping.
///
/// This operation performs a complete flattening of the function's signature by:
/// 1. Updating the types of all input parameters in the `NamedTypedList`.
/// 2. Updating the function's return type.
///
/// Any complex union types (`Type::Either`) are replaced by their corresponding
/// flattened primitive `TypeID` (pivots) as defined in the mapping.
///
/// # Parameters
/// - `atomic_function`: The `AtomicFunctionSkeleton` to modify.
/// - `map`: A mapping from union types to their unique flattened primitive type identifiers.
///
/// # Returns
/// - `Ok(())` if both parameters and return types were successfully mapped or were already primitive.
/// - `Err(LirError)` if any part of the signature uses a union type missing from the mapping.
///
/// # Implementation Note
/// This function coordinates between `typed_list` for parameters and `types` for the
/// return type to ensure the functional signature is fully ground-ready.
pub fn flatten(
    atomic_function: &mut AtomicFunctionSkeleton,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // Flatten input parameters
    typed_list::flatten_typed_variable_list(atomic_function.parameters_mut(), map)?;

    // Flatten return type
    ty::flatten(atomic_function.ty_mut(), map)?;

    Ok(())
}
