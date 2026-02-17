use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::types::typed_list;

/// Flattens all types within an `AtomicFormulaSkeleton` in place according to the provided mapping.
///
/// This function iterates through the parameters of a predicate or atomic formula skeleton
/// and replaces any complex union types (`Type::Either`) with their corresponding
/// flattened primitive `TypeID` (pivots) defined in the map.
///
/// # Parameters
/// - `atomic_formula`: The `AtomicFormulaSkeleton` (e.g., a predicate definition) to types.
/// - `map`: A mapping from union types to their unique flattened primitive type identifiers.
///
/// # Returns
/// - `Ok(())` if all parameter types were successfully mapped to pivots or were already primitive.
/// - `Err(LirError)` if a parameter uses a union type that is missing from the mapping.
///
/// # Implementation Note
/// This is a key step in domain flattening, ensuring that predicate signatures
/// match the flattened types of the objects that will be used as arguments.
pub fn flatten(
    atomic_formula: &mut AtomicFormulaSkeleton,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    typed_list::flatten_typed_variable_list(atomic_formula.parameters_mut(), map)
}
