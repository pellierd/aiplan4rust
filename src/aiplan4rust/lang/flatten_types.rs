use std::collections::HashMap;
use crate::aiplan4rust::interner::Ident;
use crate::aiplan4rust::lang::Type;

/// Trait for replacing union types with their corresponding primitive equivalents.
pub trait FlattenTypes {
    /// Applies the transformation of union types (`Type::Either`) into primitive types.
    ///
    /// # Parameters
    /// - `map`: A `HashMap` that associates each `Type::Either` with its new primitive `Ident`.
    fn flatten_types(&mut self, map: &HashMap<Type, Ident>);
}
