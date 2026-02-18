use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::typed_symbol;

/// Flattens all types in a `TypedList<VariableID, TypeID>` according to the provided mapping.
///
/// # Parameters
/// - `typed_list`: The list of typed variables to types.
/// - `map`: Mapping from union types (`Type::Either`) to their flattened type identifiers.
///
/// # Returns
/// - `Ok(())` if all types were successfully flattened or already primitive.
/// - `Err(LirError)` if any type is an `Either` not present in the mapping.
pub fn flatten_typed_variable_list(
    typed_list: &mut TypedList<VariableId, TypeId>,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    for ts in typed_list.iter_mut() {
        typed_symbol::flatten_typed_variable(ts, map)?;
    }
    Ok(())
}
