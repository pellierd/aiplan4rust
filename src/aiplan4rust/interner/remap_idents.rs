use std::collections::HashMap;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::StringID;

/// Trait for remapping identifiers (`Ident`) within a structure.
///
/// This is useful when you have a mapping from old identifiers to new ones,
/// for example after flattening types, merging domains, or linking structures,
/// and you want to update all occurrences of the identifiers consistently.
///
/// Implementors of this trait should traverse their structure and replace
/// any `Ident` according to the provided map. Conflicts or missing entries
/// should be reported via [`InternerError`].
pub trait RemapIdents {
    /// Apply a remapping of identifiers according to the provided map.
    ///
    /// # Parameters
    /// - `map`: a `HashMap` that associates each old `Ident` with a new `Ident`.
    ///
    /// # Errors
    /// Returns an [`InternerError`] in case of:
    /// - A required mapping is missing (`InternerError::MissingIdent`), or
    /// - A remap would cause a conflict (`InternerError::Conflict`).
    ///
    /// # Example
    /// ```ignore
    /// use std::collections::HashMap;
    /// use crate::aiplan4rust::interner::{Ident, InternerError, RemapIdents};
    ///
    /// let mut my_struct = ...; // some structure containing Idents
    /// let mut map: HashMap<Ident, Ident> = HashMap::new();
    /// // populate map with old -> new Idents
    ///
    /// my_struct.remap_idents(&map)?;
    /// ```
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError>;
}
