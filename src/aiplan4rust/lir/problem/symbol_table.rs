use std::cell::RefCell;
use crate::aiplan4rust::lang::ids::Id;
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use crate::aiplan4rust::lang::StringID;

/// A generic table mapping `Ident` <-> `ID`.
///
/// Works for `TypeID`, `ObjectID`, `ObjectFluentID`, `NumericFluentID`, etc.
/// Provides fast lookups in both directions.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct SymbolTable<ID: Id> {
    pub elements: Vec<StringID>,   // idx -> Ident
    pub map: HashMap<StringID, ID>, // Ident -> ID
}

impl<ID: Id> SymbolTable<ID> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            map: HashMap::new(),
        }
    }

    /// Inserts an `Ident` into the table if it does not already exist.
    ///
    /// Returns the corresponding typed ID.
    pub fn insert(&mut self, ident: StringID) -> ID {
        if let Some(&id) = self.map.get(&ident) {
            return id;
        }
        let id = ID::from(self.elements.len());
        self.elements.push(ident);
        self.map.insert(ident, id);
        id
    }

    /// Returns the ID associated with the given `Ident`, if it exists.
    pub fn get_id(&self, ident: &StringID) -> Option<ID> {
        self.map.get(ident).copied()
    }

    /// Returns the `Ident` associated with a given ID, if valid.
    pub fn get_ident(&self, id: ID) -> Option<&StringID> {
        self.elements.get(id.as_usize())
    }

    /// Checks if the table contains a given `Ident`.
    pub fn contains(&self, ident: &StringID) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = &StringID> {
        self.elements.iter()
    }

    pub fn ids(&self) -> &[StringID] {
        &self.elements
    }

    /// Attempts to get the ID for an `Ident`, returning an error if not found.
    pub fn try_get_id(&self, ident: &StringID) -> Result<ID, IndexTableError> {
        self.get_id(ident).ok_or(IndexTableError::ident_not_found(*ident))
    }

    /// Attempts to get the `Ident` for an ID, returning an error if out of bounds.
    pub fn try_get_ident(&self, id: ID) -> Result<&StringID, IndexTableError> {
        self.get_ident(id).ok_or(IndexTableError::index_out_of_bounds(id.as_usize()))
    }
}

impl<ID: Id> fmt::Display for SymbolTable<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.elements.is_empty() {
            return writeln!(f, "<empty table>");
        }

        let idx_width = self.elements.len().to_string().len();
        for (idx, ident) in self.elements.iter().enumerate() {
            // On affiche l'ID numérique du StringID puisqu'on n'a plus l'interner ici
            writeln!(f, "{:>idx_width$}: {}", idx, ident, idx_width = idx_width)?;
        }
        Ok(())
    }
}

impl<ID: Id> InternerDisplay for SymbolTable<ID> {
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        if self.elements.is_empty() {
            return writeln!(f, "<empty table>");
        }

        // Calcul des largeurs pour un joli alignement
        let idx_width = self.elements.len().to_string().len();
        let name_width = self.elements.iter()
            .map(|ident| interner.resolve_ident(*ident).unwrap_or("?").len())
            .max()
            .unwrap_or(0);

        for (idx, ident) in self.elements.iter().enumerate() {
            let name = interner.resolve_ident(*ident).unwrap_or("<unresolved>");
            writeln!(
                f,
                "{:>idx_width$}: {:<name_width$} - {}",
                idx,
                name,
                ident, // Affiche le StringID (entier)
                idx_width = idx_width,
                name_width = name_width
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum IndexTableError {
    #[error("Ident {0:?} not found in IndexTable")]
    IdentNotFound(StringID),

    #[error("Index {0} out of bounds in IndexTable")]
    IndexOutOfBounds(usize),

    #[error(transparent)]
    Interner(#[from] InternerError),
}

impl IndexTableError {
    #[track_caller]
    pub fn ident_not_found(id: StringID) -> Self {
        let err = IndexTableError::IdentNotFound(id);
        Self::log_error(&err, std::panic::Location::caller());
        err
    }

    #[track_caller]
    pub fn index_out_of_bounds(idx: usize) -> Self {
        let err = IndexTableError::IndexOutOfBounds(idx);
        Self::log_error(&err, std::panic::Location::caller());
        err
    }

    /// Helper privé pour le logging détaillé avec Backtrace
    fn log_error(err: &Self, caller: &std::panic::Location) {
        if log::log_enabled!(log::Level::Debug) {
            let bt = std::backtrace::Backtrace::force_capture();
            log::debug!(
                "\nIndexTable Error at {}:{}:{}\n{}\nStack trace:\n{}",
                caller.file(),
                caller.line(),
                caller.column(),
                err,
                bt
            );
        }
    }
}
