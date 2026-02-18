use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::LirError;

/// Flattens a `Type<TID>` in place according to the provided mapping.
///
/// # Parameters
/// - `types`: The type to modify.
/// - `map`: A mapping from union types (`Type::Either`) to their corresponding flattened type identifiers.
///
/// # Returns
/// - `Ok(())` if the type is successfully remapped or is already non-union.
/// - `Err(LirError)` if a union type cannot be remapped due to a missing mapping.
pub fn flatten(
    ty: &mut Type<TypeId>,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    if ty.is_either() {
        if let Some(&new_ident) = map.get(ty) {
            let members = ty.members_mut();
            members.clear();
            members.push(new_ident);
        } else {
            return Err(LirError::missing_type(ty.clone()));
        }
    }
    Ok(())
}
