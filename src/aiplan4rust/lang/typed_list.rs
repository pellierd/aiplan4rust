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
use crate::aiplan4rust::interner::{Ident, InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::error::LangError;
use crate::aiplan4rust::lang::{FlattenTypes, RemapIdents, Type, TypedSymbol};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lang::flatten_types::TypeFlattenError;
use crate::aiplan4rust::lang::remap_idents::RemapIdentError;

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
    /// Returns [`RemapIdentError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) -> Result<(), RemapIdentError> {
        for ts in &mut self.symbols {
            ts.remap_idents(map)?;
        }
        Ok(())
    }
}

impl FlattenTypes for TypedList {
    /// Flattens union types (`Type::Either`) in all `TypedSymbol`s of this `TypedList`
    /// according to the provided mapping. Does nothing if a type is already flattened
    /// or not present in the map.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Result<(), TypeFlattenError>` for consistency with the flattening pipeline.
    fn flatten_types(&mut self, map: &HashMap<Type, Ident>) -> Result<(), TypeFlattenError> {
        for ts in &mut self.symbols {
            ts.flatten_types(map)?;
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

/// Attempts to construct a [`TypedList`] from a [`SyntaxSubtree`] referencing an [`AstNode`]
/// and its corresponding [`SyntaxTree`].
///
/// This function expects the referenced node to represent a `TypedList`,
/// where its children are of kind `TypedItem`. Each child node is converted
/// into a [`TypedSymbol`] and added to the resulting `TypedList`.
///
/// # Parameters
///
/// - `subtree`: A reference to the `SyntaxSubtree` representing the `TypedList`.
///
/// # Returns
///
/// - `Ok(TypedList)` containing all parsed [`TypedSymbol`]s.
/// - `Err(AiplanError)` if any child fails to convert (e.g., missing node, invalid identifier).
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let typed_list = TypedList::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for TypedList {
    type Error = LangError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        let mut typed_list = TypedList::new();

        for id in node.children() {
            let child_node = ast.try_node(*id)?;
            let child_subtree = SyntaxSubtree::new(child_node, ast);
            let ty = TypedSymbol::try_from(&child_subtree)?;
            typed_list.push(ty);
        }

        Ok(typed_list)
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
