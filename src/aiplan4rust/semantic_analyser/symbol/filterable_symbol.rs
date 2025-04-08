use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, Scope, SymbolKind, Usage};
use std::fmt::{Debug, Display};

/// A trait that defines methods to access the `kind` and `scope` of a symbol.
///
/// This trait allows a unified interface for both `Declaration` and `Usage` types,
/// providing access to the kind of the symbol and its scope. It is useful for
/// grouping declarations and usages in a manner that allows filtering based on
/// symbol attributes such as type (`SymbolKind`) and scope (`Scope`).
///
/// # Required Methods
///
/// - `kind`: Returns a reference to the symbol's `SymbolKind`.
/// - `scope`: Returns a reference to the symbol's `Scope`.
///
/// This trait is implemented for both `Declaration` and `Usage` types, enabling
/// filtering or grouping based on symbol properties.
///
/// # Example
///
/// ```rust
/// let declaration = Declaration::new(...);
/// let usage = Usage::new(...);
///
/// // Both can be treated as FilterableSymbol:
/// let kind = declaration.kind();
/// let scope = usage.scope();
/// ```
pub trait FilterableSymbol: Debug + Display {
    /// Returns the `SymbolKind` of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the symbol.
    fn kind(&self) -> &SymbolKind;

    /// Returns the `Scope` of the symbol.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the symbol.
    fn scope(&self) -> &Scope;
}

// Implementing `FilterableSymbol` for the `Declaration` struct.
impl FilterableSymbol for Declaration {
    /// Returns the `SymbolKind` of the declaration.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the declaration.
    fn kind(&self) -> &SymbolKind {
        self.kind() // Delegates to the `kind` method in `Declaration`.
    }

    /// Returns the `Scope` of the declaration.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the declaration.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to the `scope` method in `Declaration`.
    }
}

// Implementing `FilterableSymbol` for the `Usage` struct.
impl FilterableSymbol for Usage {
    /// Returns the `SymbolKind` of the usage.
    ///
    /// # Returns
    ///
    /// A reference to the `SymbolKind` of the usage.
    fn kind(&self) -> &SymbolKind {
        self.kind() // Delegates to the `kind` method in `Usage`.
    }

    /// Returns the `Scope` of the usage.
    ///
    /// # Returns
    ///
    /// A reference to the `Scope` of the usage.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to the `scope` method in `Usage`.
    }
}
