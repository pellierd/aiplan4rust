//! This module defines the `Ident` type, an interned identifier used throughout the compiler.
//!
//! # Overview
//!
//! `Ident` is a lightweight wrapper around a `usize` index, used to reference interned strings
//! in a global [`StringInterner`].
//!
//! It provides:
//! - Safe handling of identifiers (`usize::MAX` is used to represent an invalid identifier).
//! - Traits to convert, display, and work with interner-aware formats.
//!
//! The interned model improves memory efficiency and comparison speed when working with large
//! volumes of identifier strings.
//!
//! # Features
//!
//! - Implements `Copy`, `Clone`, `PartialEq`, `Eq`, `Hash`, and `Serialize` / `Deserialize`
//! - Interacts with interners through the [`InternerId`], [`InternerDisplay`], and [`SyntaxDisplay`] traits
//! - Provides clear conversion with `From<usize>` and `From<Ident>`
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::interner::StringInterner;
//! use crate::aiplan4rust::interner::Ident;
//!
//! let mut interner = StringInterner::default();
//! let id = interner.intern("foo");
//!
//! assert!(id.is_valid());
//! assert_eq!(interner.resolve_ident(id), Some("foo"));
//! ```
//!
//! # Invalid Identifiers
//!
//! An `Ident` is considered invalid if its internal value is equal to `usize::MAX`. This sentinel
//! value is returned by `Ident::default()` and `Ident::invalid_value()`.
//!
//! # Related Traits
//!
//! - [`InternerId`] for abstract ID behavior
//! - [`InternerDisplay`] for interner-aware formatting
//! - [`SyntaxDisplay`] for pretty-printing with indentation
//!
//! [`StringInterner`]: crate::aiplan4rust::interner::StringInterner
//! [`InternerId`]: crate::aiplan4rust::interner::InternerId
//! [`InternerDisplay`]: crate::aiplan4rust::interner::InternerDisplay
//! [`SyntaxDisplay`]: crate::aiplan4rust::syntax::SyntaxInternerDisplay
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{InternerDisplay, InternerId, StringInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

/// An interned identifier represented by a `usize` index.
///
/// `Ident` is typically used with a `StringInterner` to refer to interned strings
/// in a compact, efficient way. It wraps a single `usize` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ident {
    value: usize,
}

impl Default for Ident {
    /// Creates an invalid `Ident` with the sentinel value `usize::MAX`.
    ///
    /// # Returns
    ///
    /// An invalid `Ident` useful for default initialization.
    fn default() -> Self {
        Ident { value: usize::MAX }
    }
}

impl Ident {
    /// Creates a new `Ident` with the specified `usize` value.
    ///
    /// # Arguments
    ///
    /// * `value` - The underlying identifier index.
    ///
    /// # Returns
    ///
    /// A new `Ident` instance.
    pub const fn new(value: usize) -> Self {
        Ident { value }
    }

    /// Returns the internal `usize` value.
    ///
    /// # Returns
    ///
    /// The raw `usize` value wrapped by this `Ident`.
    pub fn value(&self) -> usize {
        self.value
    }

    /// Converts the identifier to a `usize`.
    ///
    /// # Returns
    ///
    /// The same value as [`Self::value()`], for convenience.
    pub fn as_usize(&self) -> usize {
        self.value
    }

    /// Checks whether the identifier is valid.
    ///
    /// # Returns
    ///
    /// `true` if the identifier is not equal to [`usize::MAX`], `false` otherwise.
    pub fn is_valid(&self) -> bool {
        self.value != usize::MAX
    }

    /// Returns the sentinel value representing an invalid identifier.
    ///
    /// # Returns
    ///
    /// The constant `usize::MAX`.
    pub fn invalid_value() -> usize {
        usize::MAX
    }

    /// Remaps this `Ident` using a provided mapping table.
    ///
    /// If the identifier exists in the `map`, it is replaced by its corresponding mapped value.
    /// This is typically used during interner merging or when resolving identifier renamings
    /// between multiple contexts (e.g., domain and problem linkage).
    ///
    /// # Arguments
    ///
    /// * `map` - A mapping from old `Ident` values to new `Ident` values.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let mut id = Ident::from(1);
    /// let mut map = HashMap::new();
    /// map.insert(Ident::from(1), Ident::from(42));
    /// id.remap(&map);
    /// assert_eq!(id, Ident::from(42));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    ///
    /// # Performance
    ///
    /// This method performs a single hash map lookup and a lightweight copy operation (if found),
    /// as `Ident` is typically a `Copy` type backed by a `usize`.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
    }
}

impl InternerId for Ident {
    /// Returns the internal `usize` value.
    fn value(&self) -> usize {
        self.value()
    }

    /// Creates a new identifier from a raw `usize`.
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    /// Returns the invalid sentinel value (`usize::MAX`).
    fn invalid_value() -> usize {
        Self::invalid_value()
    }

    /// Returns whether the identifier is valid.
    fn is_valid(&self) -> bool {
        self.is_valid()
    }

    /// Returns the identifier as a `usize`.
    fn as_usize(&self) -> usize {
        self.as_usize()
    }

}

impl fmt::Display for Ident {
    /// Formats the identifier as `#<value>`, e.g., `#42`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.value)
    }
}

impl InternerDisplay for Ident {
    /// Formats the interned string corresponding to this identifier using a `StringInterner`.
    ///
    /// If the identifier has not been interned, prints a placeholder.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The `StringInterner` used to resolve the identifier.
    ///
    /// # Returns
    ///
    /// A `fmt::Result`.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(name) = interner.resolve_ident(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl SyntaxInternerDisplay for Ident {
    /// Formats the identifier with indentation and using the `StringInterner`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The interner used to resolve names.
    /// * `indent` - Indentation level (in 4-space units).
    ///
    /// # Returns
    ///
    /// A `fmt::Result`.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(name) = interner.resolve_ident(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned:{}>", *self)
        }
    }
}

impl From<usize> for Ident {
    /// Converts a `usize` into an `Ident`.
    ///
    /// # Arguments
    ///
    /// * `value` - The raw identifier value.
    ///
    /// # Returns
    ///
    /// An `Ident` wrapping the given value.
    fn from(value: usize) -> Self {
        Ident::new(value)
    }
}

impl From<Ident> for usize {
    /// Converts an `Ident` back into a `usize`.
    ///
    /// # Arguments
    ///
    /// * `ident` - The identifier to convert.
    ///
    /// # Returns
    ///
    /// The raw `usize` value.
    fn from(ident: Ident) -> usize {
        ident.value
    }
}
