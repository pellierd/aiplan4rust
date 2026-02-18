use std::collections::HashMap;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::LirError;

/// Flattens the type of a `TypedSymbol<ObjectID, TypeID>` in place according to the provided mapping.
///
/// # Parameters
/// - `symbol`: The symbol to modify.
/// - `map`: A mapping from union types (`Type::Either`) to their corresponding flattened type identifiers.
///
/// # Returns
/// - `Ok(())` if the type was successfully updated or did not need flattening.
/// - `Err(LirError)` if the type is an `Either` type that is not present in the mapping.
pub fn flatten_typed_object(
    symbol: &mut TypedSymbol<ObjectId, TypeId>,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    // Only types if it's an "either" type and exists in the map
    let ty = symbol.ty();
    if ty.is_either() {
        if let Some(&flat_type) = map.get(symbol.ty()) {
            symbol.set_ty(Type::primitive(flat_type));
        } else {
            return Err(LirError::missing_type(ty.clone()));
        }
    }
    Ok(())
}

/// Flattens the type of a `TypedSymbol<VariableID, TypeID>` in place according to the provided mapping.
///
/// # Parameters
/// - `symbol`: The symbol to modify.
/// - `map`: A mapping from union types (`Type::Either`) to their corresponding flattened type identifiers.
///
/// # Returns
/// - `Ok(())` if the type was successfully updated or did not need flattening.
/// - `Err(LirError)` if the type is an `Either` type that is not present in the mapping.
pub fn flatten_typed_variable(
    symbol: &mut TypedSymbol<VariableId, TypeId>,
    map: &HashMap<Type<TypeId>, TypeId>,
) -> Result<(), LirError> {
    let ty = symbol.ty();
    if ty.is_either() {
        if let Some(&flat_type) = map.get(symbol.ty()) {
            symbol.set_ty(Type::primitive(flat_type));
        } else {
            return Err(LirError::missing_type(ty.clone()));
        }
    }
    Ok(())
}
