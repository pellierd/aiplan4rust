//! This module defines the `Literal` type, an interned literal string used throughout the compiler.
//!
//! # Overview
//!
//! `Literal` is a lightweight wrapper around a `usize` index, used to reference interned *literal* strings
//! in a global [`StringInterner`].
//!
//! It provides:
//! - Safe handling of literal values (`usize::MAX` is used to represent an invalid literal).
//! - Traits to convert, display, and work with interner-aware formats.
//!
//! The interned model improves memory efficiency and comparison speed when working with large
//! volumes of literal strings.
//!
//! # Features
//!
//! - Implements `Copy`, `Clone`, `PartialEq`, `Eq`, `Hash`, and `Serialize` / `Deserialize`
//! - Interacts with interners through the [`InternerId`], [`InternerDisplay`], and [`SyntaxDisplay`] traits
//! - Provides clear conversion with `From<usize>` and `From<Literal>`
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::interner::StringInterner;
//! use crate::aiplan4rust::interner::Literal;
//!
//! let mut interner = StringInterner::default();
//! let lit = interner.intern_literal("42");
//!
//! assert!(lit.is_valid());
//! assert_eq!(interner.resolve_literal(lit), Some("42"));
//! ```
//!
//! # Invalid Literals
//!
//! A `Literal` is considered invalid if its internal value is equal to `usize::MAX`. This sentinel
//! value is returned by `Literal::default()` and `Literal::invalid_value()`.
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
//! [`SyntaxDisplay`]: crate::aiplan4rust::syntax::SyntaxDisplay

use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{InternerDisplay, InternerId, StringInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxDisplay};

/// An interned literal string identifier.
///
/// Wraps a `usize` index into a [`StringInterner`] that stores literal strings (e.g., numbers, constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Literal {
    value: usize,
}

impl Default for Literal {
    /// Returns a default invalid `Literal` with internal value set to `usize::MAX`.
    ///
    /// # Returns
    /// * `Literal` — a sentinel invalid literal.
    fn default() -> Self {
        Literal { value: usize::MAX }
    }
}

impl Literal {
    /// Creates a new `Literal` with the specified internal value.
    ///
    /// # Arguments
    /// * `value` — The `usize` index corresponding to an interned literal string.
    ///
    /// # Returns
    /// * `Literal` — wraps the provided index.
    pub const fn new(value: usize) -> Self {
        Literal { value }
    }

    /// Returns the internal `usize` value representing this literal.
    ///
    /// # Returns
    /// * `usize` — the internal index value.
    pub fn value(&self) -> usize {
        self.value
    }

    /// Alias for [`value()`].
    ///
    /// # Returns
    /// * `usize` — the internal index value.
    pub fn as_usize(&self) -> usize {
        self.value
    }

    /// Checks whether this `Literal` is valid (i.e., not equal to the invalid sentinel).
    ///
    /// # Returns
    /// * `bool` — `true` if valid, `false` otherwise.
    pub fn is_valid(&self) -> bool {
        self.value != usize::MAX
    }

    /// Returns the sentinel value representing an invalid literal.
    ///
    /// # Returns
    /// * `usize` — the invalid sentinel value (`usize::MAX`).
    pub fn invalid_value() -> usize {
        usize::MAX
    }

}

impl InternerId for Literal {
    /// Returns the internal `usize` index value.
    fn value(&self) -> usize {
        self.value()
    }

    /// Constructs a new `Literal` from the given index value.
    ///
    /// # Arguments
    /// * `value` — The internal index to wrap.
    ///
    /// # Returns
    /// * `Literal` — wrapping the given index.
    fn new(value: usize) -> Self {
        Self::new(value)
    }

    /// Returns the invalid sentinel index value.
    fn invalid_value() -> usize {
        Self::invalid_value()
    }

    /// Returns `true` if this `Literal` is valid.
    fn is_valid(&self) -> bool {
        self.is_valid()
    }

    /// Returns the internal index as `usize`.
    fn as_usize(&self) -> usize {
        self.as_usize()
    }
}

impl fmt::Display for Literal {
    /// Formats the literal as `"@<value>"`.
    ///
    /// # Arguments
    /// * `f` — Formatter to write output to.
    ///
    /// # Returns
    /// * `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.value)
    }
}

impl InternerDisplay for Literal {
    /// Formats the literal by resolving it via the given `StringInterner`.
    ///
    /// # Arguments
    /// * `f` — Formatter to write output to.
    /// * `interner` — Reference to the `StringInterner` to resolve the literal string.
    ///
    /// # Returns
    /// * `fmt::Result` indicating success or failure.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl SyntaxDisplay for Literal {
    /// Formats the literal with indentation for pretty-printing in syntax trees.
    ///
    /// # Arguments
    /// * `f` — Formatter to write output to.
    /// * `interner` — Reference to the `StringInterner` to resolve the literal string.
    /// * `indent` — Number of indentation levels (each level corresponds to 4 spaces).
    ///
    /// # Returns
    /// * `fmt::Result` indicating success or failure.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "<uninterned:{}>", *self)
        }
    }
}

impl From<usize> for Literal {
    /// Converts a `usize` index into a `Literal`.
    ///
    /// # Arguments
    /// * `value` — The index value to wrap.
    ///
    /// # Returns
    /// * `Literal` wrapping the provided index.
    fn from(value: usize) -> Self {
        Literal::new(value)
    }
}

impl From<Literal> for usize {
    /// Converts a `Literal` back into its internal `usize` index.
    ///
    /// # Arguments
    /// * `lit` — The `Literal` to convert.
    ///
    /// # Returns
    /// * `usize` index value.
    fn from(lit: Literal) -> usize {
        lit.value
    }
}
