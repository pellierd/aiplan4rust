use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::aiplan4rust::interner::{Ident, InternerError, StringInterner};
use std::hash::Hash;
use std::rc::Rc;
use crate::aiplan4rust::grounding::problem::ids::Id;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;


/// A generic table mapping `Ident` <-> `ID`.
///
/// Works for `TypeID`, `ObjectID`, `ObjectFluentID`, `NumericFluentID`, etc.
/// Provides fast lookups in both directions.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct SymbolTable<ID: Id> {
    elements: Vec<Ident>,   // idx -> Ident
    map: HashMap<Ident, ID>, // Ident -> ID
    /// Reference-counted interner
    #[serde(skip)]
    interner: Rc<StringInterner>,
}

impl<ID: Id> SymbolTable<ID> {
    pub fn new(interner: Rc<StringInterner>) -> Self {
        Self {
            elements: Vec::new(),
            map: HashMap::new(),
            interner
        }
    }

    /// Inserts an `Ident` into the table if it does not already exist.
    ///
    /// Returns the corresponding typed ID.
    pub fn insert(&mut self, ident: Ident) -> ID {
        if let Some(&id) = self.map.get(&ident) {
            return id;
        }
        let id = ID::from_usize(self.elements.len());
        self.elements.push(ident);
        self.map.insert(ident, id);
        id
    }

    /// Returns the ID associated with the given `Ident`, if it exists.
    pub fn get_id(&self, ident: &Ident) -> Option<ID> {
        self.map.get(ident).copied()
    }

    /// Returns the `Ident` associated with a given ID, if valid.
    pub fn get_ident(&self, id: ID) -> Option<&Ident> {
        self.elements.get(id.as_usize())
    }

    /// Checks if the table contains a given `Ident`.
    pub fn contains(&self, ident: &Ident) -> bool {
        self.map.contains_key(ident)
    }

    /// Returns the number of elements in the table.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over all `Ident`s in the table.
    pub fn iter(&self) -> impl Iterator<Item = &Ident> {
        self.elements.iter()
    }

    /// Attempts to get the ID for an `Ident`, returning an error if not found.
    pub fn try_get_id(&self, ident: &Ident) -> Result<ID, IndexTableError> {
        self.get_id(ident).ok_or(IndexTableError::ident_not_found(*ident))
    }

    /// Attempts to get the `Ident` for an ID, returning an error if out of bounds.
    pub fn try_get_ident(&self, id: ID) -> Result<&Ident, IndexTableError> {
        self.get_ident(id).ok_or(IndexTableError::index_out_of_bounds(id.as_usize()))
    }

    /// Résout un `ID` en une chaîne de caractères, si possible.
    pub fn get_string(&self, id: ID) -> Option<&str> {
        self.get_ident(id)
            .and_then(|ident| self.interner.resolve_ident(*ident))
    }

    /// Résout un `ID` en une chaîne de caractères, renvoie une erreur si impossible.
    pub fn try_get_string(&self, id: ID) -> Result<&str, IndexTableError> {
        let ident = self.try_get_ident(id)?; // utilise try_get_ident pour l'erreur si invalide
        Ok(self.interner.try_resolve_ident(*ident)?)
    }

    /// Réinjecte un interner après désérialisation
    pub fn set_interner(&mut self, interner: Rc<StringInterner>) {
        self.interner = interner;
    }
}

impl<ID: Id> fmt::Display for SymbolTable<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, ident) in self.elements.iter().enumerate() {
            let name = self.interner.resolve_ident(*ident).unwrap_or("<unresolved>");
            writeln!(f, "{}: {} ({})", idx, name, ident.as_usize())?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum IndexTableError {
    #[error("Ident {0:?} not found in IndexTable")]
    IdentNotFound(Ident),

    #[error("Index {0} out of bounds in IndexTable")]
    IndexOutOfBounds(usize),

    #[error(transparent)]
    Interner(#[from] InternerError),
}

impl IndexTableError {
    pub fn ident_not_found(id: Ident) -> Self {
        log::debug!("Creating IndexTableError::IdentNotFound for {:?}", id);
        IndexTableError::IdentNotFound(id)
    }

    pub fn index_out_of_bounds(idx: usize) -> Self {
        log::debug!("Creating IndexTableError::IndexOutOfBounds for {}", idx);
        IndexTableError::IndexOutOfBounds(idx)
    }
}
