//! Module defining `TypedList`, a collection of typed symbols.
//!
//! This module provides the `TypedList` struct, which is a wrapper around a vector of
//! `TypedSymbol` items. It offers convenient management and access of typed symbols
//! in a PDDL-like syntax domain context.
//!
//! `TypedList` supports common collection operations through `Deref` to the underlying
//! slice, making iteration and manipulation ergonomic.
//!
//! # Example
//!
//! ```rust
//! use crate::TypedList;
//! use crate::TypedSymbol;
//!
//! let mut list = TypedList::new();
//! let typed_symbol = TypedSymbol::new(/* ... */);
//! list.push(typed_symbol);
//! assert_eq!(list.len(), 1);
//! for symbol in list.iter() {
//!     // process each TypedSymbol
//! }
//! ```

use std::collections::HashMap;
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::{RemapIdents, StringID, TypedSymbol};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// A list of `TypedSymbol` items.
///
/// This struct wraps a `Vec<TypedSymbol>` and provides
/// convenient access to the underlying slice via dereferencing.
///
/// # Examples
///
/// ```
/// let mut list = TypedList::new();
/// list.push(typed_symbol);
/// assert_eq!(list.len(), 1);
/// for symbol in list.iter() {
///     // use symbol
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList {
    symbols: Vec<TypedSymbol>,
}

impl TypedList {
    /// Creates a new, empty `TypedSymbolList`.
    ///
    /// # Returns
    ///
    /// A fresh `TypedSymbolList` containing no elements.
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    /// Create a TypedList from a vector of TypedSymbol
    ///
    /// # Arguments
    /// * `symbols` - A vector containing the TypedSymbol instances to include in the list
    ///
    /// # Returns
    /// A new `TypedList` containing the provided symbols
    pub fn from_symbols(symbols: Vec<TypedSymbol>) -> Self {
        Self { symbols }
    }


    /// Returns an empty instance of the type_checker.
    ///
    /// This is a convenience method that creates a default (empty) value.
    /// It relies on the `Default` trait implementation for this type_checker.
    ///
    /// # Examples
    ///
    /// ```
    /// let empty_list = TypedList::empty();
    /// assert!(empty_list.is_empty()); // supposant que is_empty() est défini
    /// ```
    pub fn empty() -> Self {
        Self::default()
    }

}

impl RemapIdents for TypedList {
    /// Remaps all atomic identifiers of the `TypedSymbol`s contained in this `TypedList`
    /// according to the provided mapping table.
    ///
    /// Each `TypedSymbol` in `self.symbols` is updated: if its symbol or associated types
    /// exist as keys in `map`, they are replaced with the corresponding values.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap` mapping old `Ident`s to new `Ident`s.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        for ts in &mut self.symbols {
            ts.remap_idents(map)?;
        }
        Ok(())
    }
}

impl Deref for TypedList {
    type Target = Vec<TypedSymbol>;

    /// Dereferences the `TypedList` to a `Vec<TypedSymbol>`.
    ///
    /// This allows using all methods of `Vec<TypedSymbol>` directly on
    /// `TypedList` instances, including methods like `push`, `pop`, `len`, and more.
    fn deref(&self) -> &Self::Target {
        &self.symbols
    }
}
/// Allows consuming iteration over `TypedList`, yielding owned `TypedSymbol`s.
///
/// This implementation enables using `TypedList` in a `for` loop or
/// passing it to functions expecting an iterator of `TypedSymbol`,
/// consuming the list in the process.
///
/// # Example
///
/// ```
/// let typed_list: TypedList = ...;
/// for symbol in typed_list {
///     // symbol is a TypedSymbol
/// }
/// ```
impl IntoIterator for TypedList {
    type Item = TypedSymbol;
    type IntoIter = std::vec::IntoIter<TypedSymbol>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.into_iter()
    }
}

/// Allows borrowed iteration over `TypedList`, yielding references to `TypedSymbol`s.
///
/// This implementation enables iterating over a borrowed `TypedList` without consuming it.
///
/// # Example
///
/// ```
/// let typed_list: &TypedList = ...;
/// for symbol in typed_list {
///     // symbol is a &TypedSymbol
/// }
/// ```
impl<'a> IntoIterator for &'a TypedList {
    type Item = &'a TypedSymbol;
    type IntoIter = std::slice::Iter<'a, TypedSymbol>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.iter()
    }
}

impl DerefMut for TypedList {
    /// Dereferences the `TypedList` mutably to a `Vec<TypedSymbol>`.
    ///
    /// This allows mutating the underlying vector, enabling calls to
    /// mutable methods like `push`, `clear`, `sort`, etc.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.symbols
    }
}

/// Implements `Display` for `TypedList`.
///
/// This implementation formats the `TypedList` as a space-separated list of symbols
/// enclosed in parentheses.
///
/// Each symbol is displayed using its own `Display` implementation, without
/// any additional processing or name resolution.
///
/// # Example
///
/// ```text
/// (sym1 sym2 sym3)
/// ```
///
/// # Behavior
///
/// - Starts by writing an opening parenthesis `(`.
/// - Iterates over all symbols in the list.
/// - Writes each symbol separated by a space.
/// - Ends with a closing parenthesis `)`.
impl fmt::Display for TypedList {
    /// Formats the `TypedList` for display.
    ///
    /// Writes the list of symbols in parentheses, separated by spaces.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{sym}")?;
            first = false;
        }
        write!(f, ")")
    }
}

/// Implements `DisplayWithInterner` for `TypedList`.
///
/// A `TypedList` is a collection of `TypedSymbol`s, each potentially associated
/// with a type_checker. The output is a parenthesized, space-separated list of symbols
/// with their types, suitable for debugging or display.
///
/// # Example
///
/// ```text
/// (?x - location ?y - (either robot vehicle))
/// ```
impl InternerDisplay for TypedList {
    /// Formats the `TypedList` using the provided `StringInterner`.
    ///
    /// This writes the list in PDDL-style syntax:
    /// - Starts with an opening parenthesis `(`.
    /// - Symbols are separated by spaces.
    /// - Each symbol is formatted via `TypedSymbol::fmt_with`.
    /// - Ends with a closing parenthesis `)`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The interner to resolve symbol names.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            sym.fmt_with_interner(f, interner)?;
            first = false;
        }
        write!(f, ")")
    }
}

/// Displays a `TypedList` in PDDL syntax.
///
/// A `TypedList` is a list of `TypedSymbol`s, each with optional types.
/// The output is formatted as a parenthesized, space-separated list of typed symbols.
///
/// This implementation is intended for generating PDDL-compatible representations
/// of variables or objects with their declared types.
///
/// # Example
///
/// ```text
/// (?x - location ?y - (either robot vehicle))
/// ```
impl SyntaxInternerDisplay for TypedList {
    /// Formats the `TypedList` in PDDL syntax.
    ///
    /// This function writes:
    /// - An opening parenthesis `(`.
    /// - Each `TypedSymbol`, separated by spaces.
    /// - A closing parenthesis `)`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    /// * `interner` - The `StringInterner` used to resolve symbol and type_checker names.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting succeeded.
    ///
    /// # Example Output
    ///
    /// ```text
    /// (?x - location ?y - robot)
    /// ```
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        let mut first = true;
        for sym in &self.symbols {
            if !first {
                write!(f, " ")?;
            }
            sym.fmt_syntax_with_interner(f, interner)?;
            first = false;
        }
        Ok(())
    }
}
