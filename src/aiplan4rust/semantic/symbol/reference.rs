use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::elements::Ident;
use std::fmt;

/// Represents a reference to a declared symbol, with its identifier and kind.
///
/// This is typically extracted from an AST node during semantic analysis and used
/// to describe the declaration of a symbol (e.g., constant, predicate, action).
///
/// A `Reference` is immutable, hashable, and suitable for use in maps or sets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reference {
    ident: Ident,
    kind: SymbolKind,
}

impl Reference {
    /// Creates a new `Reference` from an identifier and symbol kind.
    ///
    /// # Parameters
    /// - `ident`: The identifier of the symbol (interned string).
    /// - `kind`: The kind/category of the symbol (e.g., `Predicate`, `Action`, etc.).
    ///
    /// # Returns
    /// A new `Reference` instance.
    pub fn new(ident: Ident, kind: SymbolKind) -> Self {
        Self { ident, kind }
    }

    /// Returns the [`Ident`] of the symbol.
    pub fn ident(&self) -> Ident {
        self.ident
    }

    /// Returns the [`SymbolKind`] of the symbol.
    pub fn kind(&self) -> SymbolKind {
        self.kind
    }
}

impl fmt::Display for Reference {
    /// Formats the symbol reference as `"<kind> <ident>"`, e.g., `"Predicate at(s, l)"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.ident)
    }
}
