use std::collections::HashMap;
use crate::aiplan4rust::interner::Ident;
use crate::aiplan4rust::lang::{Type, TypedList, TypedSymbol};
use crate::aiplan4rust::lir::LirError;

/// Trait for replacing union types (`Type::Either`) with their corresponding primitive equivalents.
///
/// This trait is intended for **strict / complete remapping**: after calling
/// `remap_types`, all union types in the object should be replaced by their
/// primitive identifiers according to the provided map. Any union type that
/// cannot be remapped will produce a LIR-level error.
///
/// # Examples
/// ```ignore
/// let mut ty: Type = Type::Either(vec![...]);
/// let map: HashMap<Type, Ident> = ...;
/// // After this call, ty contains only primitive types.
/// ty.remap_types(&map)?;
/// ```
pub trait RemapTypes {
    /// Replaces union types (`Type::Either`) with primitive types according to the provided mapping.
    ///
    /// - Union types must be replaced by their corresponding primitive identifiers.
    /// - Non-union types are left unchanged.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping each union type (`Type::Either`) to its corresponding primitive `Ident`.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped or are already primitive.
    /// - `Err(LirError)` if at least one union type cannot be remapped (strict behavior).
    fn remap_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), LirError>;
}


/// Implements the `RemapTypes` trait for `Type`.
///
/// This implementation performs a **strict / complete remapping**:
/// - If the type is a union type (`Type::Either`), it is replaced by its
///   corresponding primitive identifier according to the provided map.
/// - If the union type is not present in the map, a LIR-level error is returned.
/// - Non-union types are left unchanged.
///
/// # Parameters
/// - `map`: A `HashMap<Type, Ident>` mapping union types (`Type::Either`) to their
///   corresponding primitive identifiers.
///
/// # Returns
/// - `Ok(())` if the type is successfully remapped or is already non-union.
/// - `Err(LirError)` if a union type cannot be remapped due to a missing mapping.
impl RemapTypes for Type {
    fn remap_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), LirError> {
        if self.is_either() {
            if let Some(new_ident) = map.get(&self) {
                // Clear the current members and replace with the mapped primitive.
                let members = self.members_mut();
                members.clear();
                members.push(*new_ident);
            } else {
                // Strict remapping: fail if the mapping is missing.
                return Err(LirError::missing_type(self.clone()));
            }
        }
        Ok(())
    }
}

impl RemapTypes for TypedList {
    /// Flattens union types (`Type::Either`) in all `TypedSymbol`s of this `TypedList`
    /// according to the provided mapping.
    ///
    /// Does nothing for symbols whose types are already flattened or
    /// for union types not present in the map.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their
    ///   corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all symbol types are successfully remapped or no changes are needed.
    /// - `Err(LirError)` if an error occurs while remapping any symbol’s type.
    fn remap_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), LirError> {
        for ts in self.iter_mut() {
            ts.remap_types(map)?;
        }
        Ok(())
    }
}


impl RemapTypes for TypedSymbol {
    /// Flattens union types (`Type::Either`) in the symbol’s type
    /// according to the provided mapping.
    ///
    /// Does nothing if the type is already flattened or if no matching
    /// union type is present in the map.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their
    ///   corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if the remapping succeeds or no change is needed.
    /// - `Err(LirError)` if an error occurs while remapping the symbol’s type.
    fn remap_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), LirError> {
        self.ty_mut().remap_types(map)?;
        Ok(())
    }
}
