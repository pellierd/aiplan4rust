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

use std::collections::HashMap;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::lang::Ident;
use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

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

    /// Remaps the symbol's identifier based on the provided mapping.
    ///
    /// If the current identifier exists as a key in the `map`, it will be replaced
    /// by the corresponding mapped identifier. Otherwise, it remains unchanged.
    ///
    /// # Parameters
    ///
    /// - `map`: A reference to a `HashMap` that maps old `Ident` values to new `Ident` values.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut symbol = Symbol { ident: old_ident, kind: some_kind };
    /// let mut mapping = HashMap::new();
    /// mapping.insert(old_ident, new_ident);
    /// symbol.remap_idents(&mapping);
    /// ```
    ///
    /// After calling this method, `symbol.ident` will be updated to `new_ident` if
    /// `old_ident` was present in the mapping.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.ident().remap_idents(map);
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

impl InternerDisplay for Symbol {
    /// Formats the symbol by resolving its identifier name using the provided interner.
    ///
    /// If the identifier can be resolved via the interner, the resolved string is printed.
    /// Otherwise, a fallback string of the form `"unknown(<ident>)"` is displayed.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter to write the output to.
    /// - `interner`: Reference to a `StringInterner` used to resolve identifiers.
    ///
    /// # Returns
    ///
    /// Returns `fmt::Result` indicating success or failure of the write operation.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        match interner.resolve_ident(self.ident()) {
            Some(resolved) => write!(f, "{}", resolved),
            None => write!(f, "unknown({})", self.ident()),
        }
    }
}

impl SyntaxInternerDisplay for Symbol {
    /// Formats the symbol with indentation, resolving the identifier name using
    /// the provided interner.
    ///
    /// This method writes the specified indentation, then delegates the actual
    /// name formatting to `InternerDisplay::fmt_with_interner` to avoid code duplication.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter to write the output to.
    /// - `interner`: Reference to a `StringInterner` used to resolve identifiers.
    /// - `indent`: Number of spaces to indent the output.
    ///
    /// # Returns
    ///
    /// Returns `fmt::Result` indicating success or failure of the write operation.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        // Write indentation spaces
        write_indent(f, indent)?;
        // Delegate to InternerDisplay implementation to format the symbol name
        self.fmt_with_interner(f, interner)
    }
}
