use thiserror::Error;
use std::collections::HashMap;
use crate::aiplan4rust::interner::Ident;

/// Trait for remapping identifiers (`Ident`) within a structure.
///
/// This is useful when you have a mapping from old identifiers to new ones,
/// e.g., after flattening types or merging domains, and you want to update
/// all occurrences of the identifiers consistently.
pub trait RemapIdents {
    /// Apply a remapping of identifiers according to the provided map.
    ///
    /// # Parameters
    /// - `map`: a `HashMap` that associates each old `Ident` with a new `Ident`.
    ///
    /// # Errors
    /// Returns `RemapIdentError` if a required mapping is missing or
    /// if a remap would cause a conflict.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) -> Result<(), RemapIdentError>;
}

/// Errors that can occur during identifier remapping.
#[derive(Debug, Error)]
pub enum RemapIdentError {
    /// Indicates that a key in the structure has no corresponding entry in the remap table.
    #[error("Missing remap for identifier {0:?}")]
    MissingMapping(Ident),

    /// Indicates that two keys in the structure would be remapped to the same identifier.
    #[error("Remapped identifier conflict for {0:?}")]
    Conflict(Ident),
}

impl RemapIdentError {
    /// Creates a `MissingMapping` error for the given `Ident`.
    pub fn missing_mapping(id: Ident) -> Self {
        RemapIdentError::MissingMapping(id)
    }

    /// Creates a `Conflict` error for the given `Ident`.
    pub fn conflict(id: Ident) -> Self {
        RemapIdentError::Conflict(id)
    }
}
