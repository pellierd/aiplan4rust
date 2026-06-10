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

use crate::aiplan4rust::compiler::semantic::symbol::SymbolKind;
use crate::aiplan4rust::compiler::syntax::{write_indent, SyntaxInternerDisplay};
use crate::aiplan4rust::support::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::support::lang::{RemapSymbol, SymbolId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    ident: SymbolId,
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
    pub fn new(ident: SymbolId, kind: SymbolKind) -> Self {
        Self { ident, kind }
    }

    /// Returns the identifier (`Ident`) of the symbol.
    pub fn id(&self) -> SymbolId {
        self.ident
    }

    /// Sets a new identifier for this symbol.
    ///
    /// # Arguments
    ///
    /// * `ident` — The new identifier to assign.
    pub fn set_ident(&mut self, ident: SymbolId) {
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

impl RemapSymbol for Symbol {
    /// Remaps the symbol's identifier according to the provided mapping.
    ///
    /// If the symbol's current `Ident` is a key in `map`, it is replaced by
    /// the corresponding value. Otherwise, the identifier remains unchanged.
    ///
    /// # Parameters
    ///
    /// - `map` – A `HashMap` mapping old `Ident`s to new `Ident`s.
    ///
    /// # Errors
    ///
    /// Returns `InternerError` if the remapping fails (propagated from inner calls).
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        self.id().remap_idents(map)?;
        Ok(())
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
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        match interner.resolve_symbol(self.id()) {
            Some(resolved) => write!(f, "{}", resolved),
            None => write!(f, "unknown({})", self.id()),
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
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        // Write indentation spaces
        write_indent(f, indent)?;
        // Delegate to InternerDisplay implementation to format the symbol name
        self.fmt_with_interner(f, interner)
    }
}
