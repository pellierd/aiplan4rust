//! Provides the `Filterable` trait for accessing common properties of symbols,
//! such as their kind and scope, enabling unified handling of symbol declarations and usages.
//!
//! This module defines the `Filterable` trait, which abstracts over different symbol types
//! (`Declaration` and `Usage`) to allow consistent access to their `SymbolKind` and `Scope`.
//! This facilitates operations like filtering or grouping symbols by these attributes,
//! which is useful in semantic analysis, symbol resolution, and diagnostics.

use crate::aiplan4rust::compiler::semantic::symbol::{Declaration, Scope, SymbolKind, Usage};
use std::fmt::{Debug, Display};

/// A trait for accessing key properties of symbols, specifically their kind and scope.
///
/// The `Filterable` trait provides a common interface for symbol-related types,
/// allowing code to treat `Declaration` and `Usage` uniformly when filtering or analyzing
/// symbols by their kind (`SymbolKind`) and their lexical or semantic scope (`Scope`).
///
/// # Required Methods
///
/// - `kind()`: Returns the symbol's kind (`SymbolKind`).
/// - `scope()`: Returns a reference to the symbol's scope (`Scope`).
///
/// # Implementors
///
/// This trait is implemented for:
/// - [`Declaration`]: Represents a symbol declaration in the semantic model.
/// - [`Usage`]: Represents a usage/reference of a symbol in the semantic model.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::semantic::symbol::{Declaration, Usage};
/// use crate::aiplan4rust::semantic::symbol::Filterable;
///
/// let declaration: Declaration = /* ... */;
/// let usage: Usage = /* ... */;
///
/// // Both Declaration and Usage can be treated as Filterable.
/// let decl_kind = declaration.kind();
/// let usage_scope = usage.scope();
/// ```
pub trait Filterable: Debug + Display {
    /// Returns the `SymbolKind` of the symbol.
    ///
    /// # Returns
    ///
    /// The kind/category of the symbol.
    fn kind(&self) -> SymbolKind;

    /// Returns the `Scope` of the symbol.
    ///
    /// # Returns
    ///
    /// The lexical or semantic scope in which the symbol is valid.
    fn scope(&self) -> &Scope;
}

// Implementing `Filterable` for the `Declaration` struct.
impl Filterable for Declaration {
    /// Returns the `SymbolKind` of the declaration.
    ///
    /// # Returns
    ///
    /// The kind/category of the declared symbol.
    fn kind(&self) -> SymbolKind {
        self.symbol_kind() // Delegates to Declaration's own method.
    }

    /// Returns the `Scope` of the declaration.
    ///
    /// # Returns
    ///
    /// The scope in which the symbol declaration is valid.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to Declaration's own method.
    }
}

// Implementing `Filterable` for the `Usage` struct.
impl Filterable for Usage {
    /// Returns the `SymbolKind` of the usage.
    ///
    /// # Returns
    ///
    /// The kind/category of the symbol usage.
    fn kind(&self) -> SymbolKind {
        self.symbol_kind() // Delegates to Usage's own method.
    }

    /// Returns the `Scope` of the usage.
    ///
    /// # Returns
    ///
    /// The scope in which the symbol usage occurs.
    fn scope(&self) -> &Scope {
        self.scope() // Delegates to Usage's own method.
    }
}
