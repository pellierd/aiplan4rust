use std::collections::HashMap;
use serde::{Deserialize, Serialize};
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

    /// Retrieves a mutable reference to an `Ident` by its value.
    ///
    /// # Parameters
    /// - `id`: Reference to the `Ident` to access mutably.
    ///
    /// # Returns
    /// `Some(&mut Ident)` if the `Ident` exists, or `None` otherwise.
    pub fn get_mut(&mut self, id: &Ident) -> Option<&mut Ident> {
        if let Some(&idx) = self.map.get(id) {
            Some(&mut self.elements[idx])
        } else {
            None
        }
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
