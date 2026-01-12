use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::aiplan4rust::interner::Ident;

/// A table mapping `Ident` to indices (`usize`) and vice versa.
///
/// This structure allows fast lookup of an `Ident` by index, and index by `Ident`.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct IndexTable {
    elements: Vec<Ident>,       // idx -> Ident
    map: HashMap<Ident, usize>, // Ident -> idx
}

impl IndexTable {
    /// Creates a new, empty `IndexTable`.
    ///
    /// # Returns
    /// A new instance of `IndexTable` with no elements.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            map: HashMap::new(),
        }
    }

    /// Inserts an `Ident` into the table if it does not already exist.
    ///
    /// # Parameters
    /// - `id`: The `Ident` to insert.
    ///
    /// # Returns
    /// The index (`usize`) of the `Ident` in the table.
    pub fn insert(&mut self, id: Ident) -> usize {
        if let Some(&idx) = self.map.get(&id) {
            return idx;
        }
        let idx = self.elements.len();
        self.elements.push(id);
        self.map.insert(id, idx);
        idx
    }

    /// Retrieves the index of a given `Ident`.
    ///
    /// # Parameters
    /// - `id`: Reference to the `Ident` to look up.
    ///
    /// # Returns
    /// `Some(index)` if the `Ident` exists, or `None` otherwise.
    pub fn get_index(&self, id: &Ident) -> Option<usize> {
        self.map.get(id).copied()
    }

    /// Returns the index of the given `Ident` in the `IndexTable`.
    ///
    /// # Parameters
    /// - `id`: The identifier to look up.
    ///
    /// # Returns
    /// - `Ok(index)` if the identifier exists in the table.
    /// - `Err(IndexTableError::IdentNotFound)` if the identifier is absent.
    ///
    /// # Example
    /// ```rust
    /// # use aiplan4rust::interner::Ident;
    /// # use aiplan4rust::grounding::problem::{IndexTable, IndexTableError};
    /// let mut table = IndexTable::new();
    /// let id = Ident(42);
    /// table.insert(id);
    /// assert_eq!(table.try_index(&id).unwrap(), 0);
    /// assert!(table.try_index(&Ident(99)).is_err());
    /// ```
    pub fn try_index(&self, id: &Ident) -> Result<usize, IndexTableError> {
        self.get_index(id).ok_or(IndexTableError::ident_not_found(*id))
    }

    /// Retrieves the `Ident` at a given index.
    ///
    /// # Parameters
    /// - `idx`: Index of the element to retrieve.
    ///
    /// # Returns
    /// `Some(&Ident)` if the index is valid, or `None` otherwise.
    pub fn get_ident(&self, idx: usize) -> Option<&Ident> {
        self.elements.get(idx)
    }

    /// Returns a reference to the `Ident` at the given index in the `IndexTable`.
    ///
    /// # Parameters
    /// - `idx`: The index to look up.
    ///
    /// # Returns
    /// - `Ok(&Ident)` if the index is valid.
    /// - `Err(IndexTableError::IndexOutOfBounds)` if the index is out of range.
    ///
    /// # Example
    /// ```rust
    /// # use aiplan4rust::interner::Ident;
    /// # use aiplan4rust::grounding::problem::{IndexTable, IndexTableError};
    /// let mut table = IndexTable::new();
    /// let id = Ident(42);
    /// table.insert(id);
    ///
    /// assert_eq!(table.try_get_ident(0).unwrap(), &id);
    /// assert!(matches!(table.try_get_ident(1), Err(IndexTableError::IndexOutOfBounds(1))));
    /// ```
    pub fn try_get_ident(&self, idx: usize) -> Result<&Ident, IndexTableError> {
        self.get_ident(idx)
            .ok_or(IndexTableError::index_out_of_bounds(idx))
    }

    /// Checks whether a given `Ident` exists in the table.
    ///
    /// # Parameters
    /// - `id`: Reference to the `Ident` to check.
    ///
    /// # Returns
    /// `true` if the `Ident` exists, `false` otherwise.
    pub fn contains(&self, id: &Ident) -> bool {
        self.map.contains_key(id)
    }

    /// Returns the number of elements in the table.
    ///
    /// # Returns
    /// Number of elements (`usize`).
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Checks whether the table is empty.
    ///
    /// # Returns
    /// `true` if the table contains no elements, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over all `Ident`s in the table.
    ///
    /// # Returns
    /// An iterator of `&Ident`.
    pub fn iter(&self) -> impl Iterator<Item = &Ident> {
        self.elements.iter()
    }
}

#[derive(Debug, Error)]
pub enum IndexTableError {
    #[error("Ident {0:?} not found in IndexTable")]
    IdentNotFound(Ident),

    #[error("Index {0} out of bounds in IndexTable")]
    IndexOutOfBounds(usize),
}

impl IndexTableError {
    /// Creates an `IdentNotFound` error for the given `Ident`.
    /// Logs a debug message with the identifier.
    pub fn ident_not_found(id: Ident) -> Self {
        log::debug!("Creating IndexTableError::IdentNotFound for {:?}", id);
        IndexTableError::IdentNotFound(id)
    }

    /// Creates an `IndexOutOfBounds` error for the given index.
    /// Logs a debug message with the index.
    pub fn index_out_of_bounds(idx: usize) -> Self {
        log::debug!("Creating IndexTableError::IndexOutOfBounds for {}", idx);
        IndexTableError::IndexOutOfBounds(idx)
    }
}
