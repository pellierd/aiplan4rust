//! Module `symbol`.
//!
//! This module defines the [`Symbol`] struct, representing a reference to a declared symbol,
//! including its identifier and kind (`SymbolKind`).
//!
//! A `Symbol` is typically extracted during semantic analysis of an Abstract Syntax Tree (AST)
//! and is used to represent the declaration of a symbol such as a constant, predicate, action, etc.
//!
//! The struct is immutable (except when explicitly modified via setters), hashable,
//! and suitable for use as a key in maps or sets.

use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::lang::Ident;
use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents a reference to a declared symbol, consisting of its identifier and kind.
///
/// A `Symbol` encapsulates an identifier (`Ident`) and a symbol kind (`SymbolKind`) that describes
/// the nature of the symbol (e.g., predicate, action, constant).
///
/// This struct supports equality, hashing, cloning, and serialization.
///
/// # Example
///
/// ```
/// use your_crate::{Symbol, SymbolKind, Ident};
///
/// let sym = Symbol::new(Ident::new("at"), SymbolKind::Predicate);
/// println!("{}", sym); // prints: Predicate at
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    ident: Ident,
    kind: SymbolKind,
}

impl Symbol {
    /// Creates a new symbol reference from an identifier and a symbol kind.
    ///
    /// # Arguments
    ///
    /// * `ident` — The symbol's identifier (interned string).
    /// * `kind` — The symbol kind (`SymbolKind`), for example `Predicate`, `Action`, etc.
    ///
    /// # Returns
    ///
    /// A new [`Symbol`] instance.
    pub fn new(ident: Ident, kind: SymbolKind) -> Self {
        Self { ident, kind }
    }

    /// Returns the identifier (`Ident`) of the symbol.
    pub fn ident(&self) -> Ident {
        self.ident
    }

    /// Sets a new identifier for this symbol.
    ///
    /// # Arguments
    ///
    /// * `ident` — The new identifier to assign.
    pub fn set_ident(&mut self, ident: Ident) {
        self.ident = ident;
    }

    /// Returns the kind (`SymbolKind`) of the symbol.
    pub fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// Sets a new kind for this symbol.
    ///
    /// # Arguments
    ///
    /// * `kind` — The new symbol kind to assign.
    pub fn set_kind(&mut self, kind: SymbolKind) {
        self.kind = kind;
    }
}

impl fmt::Display for Symbol {
    /// Formats the symbol reference as `"<kind> <ident>"`.
    ///
    /// For example, a symbol of kind `Predicate` with identifier `at` will be formatted as `"Predicate at"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.ident)
    }
}
