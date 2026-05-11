//! Module `syntax_display`
//!
//! This module defines two related traits for formatting Rust structures into
//! syntax strings:
//!
//! 1. [`SyntaxInternerDisplay`] — for types that require a `StringInterner` and
//!    optional indentation to render their syntax.
//! 2. [`SyntaxDisplay`] — for types that can render themselves as syntax strings
//!    without an interner or indentation.
//!
//! These traits provide a unified interface for converting complex ASTs, domain
//! definitions, or problems into human-readable syntax strings, with optional
//! control over formatting details.
//!
//! # Examples
//!
//! Using `SyntaxInternerDisplay`:
//!
//! ```rust
//! use crate::aiplan4rust::interner::StringInterner;
//! use std::fmt;
//!
//! struct MyType {
//!     id: usize,
//! }
//!
//! impl crate::aiplan4rust::syntax_display::SyntaxInternerDisplay for MyType {
//!     fn fmt_syntax_with_interner_and_indent(
//!         &self,
//!         f: &mut fmt::Formatter<'_>,
//!         interner: &StringInterner,
//!         indent: usize,
//!     ) -> fmt::Result {
//!         let indent_str = "  ".repeat(indent);
//!         write!(f, "{}{}", indent_str, interner.resolve(self.id))
//!     }
//! }
//! ```
//!
//! Using `SyntaxDisplay` for simpler types:
//!
//! ```rust
//! use std::fmt;
//! struct SimpleType {
//!     name: String,
//! }
//!
//! impl crate::aiplan4rust::syntax_display::SyntaxDisplay for SimpleType {
//!     fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "SimpleType({})", self.name)
//!     }
//! }
//!
//! let value = SimpleType { name: "example".to_string() };
//! assert_eq!(value.to_syntax_string(), "SimpleType(example)");
//! ```

use crate::aiplan4rust::interner::SymbolInterner;
use std::fmt;
use std::fmt::{Formatter, Write};

/// Default number of characters used per indentation level.
///
/// This constant is used by [`SyntaxInternerDisplay`] implementations as the debug
/// indent width, typically 2 spaces.
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Default character used for indentation.
///
/// This constant is used by [`SyntaxInternerDisplay`] implementations as the debug
/// indent character, typically a space `' '`.
pub const DEFAULT_INDENT_CHAR: char = ' ';

/// Writes indentation to the provided formatter.
///
/// The indentation written consists of `level * DEFAULT_INDENT_WIDTH`
/// occurrences of the `DEFAULT_INDENT_CHAR`.
///
/// # Arguments
///
/// * `f` - The formatter to write to.
/// * `level` - The indentation level (number of indent units).
///
/// # Errors
///
/// Returns any error encountered while writing to the formatter.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// let mut s = String::new();
/// write_indent(&mut s, 3).unwrap();
/// assert_eq!(s, "      "); // 6 spaces if DEFAULT_INDENT_WIDTH=2
/// ```
pub fn write_indent(f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
    for _ in 0..(level * DEFAULT_INDENT_WIDTH) {
        write!(f, "{}", DEFAULT_INDENT_CHAR)?;
    }
    Ok(())
}

/// Trait for formatting values with planned syntax, supporting indentation and interner resolution.
///
/// Provides debug constants for indentation width and indent character,
/// along with methods to format the value at variable indentation levels.
///
/// # Default constants
///
/// - [`DEFAULT_INDENT_WIDTH`]: number of characters per indent level (debug: 2).
/// - [`DEFAULT_INDENT_CHAR`]: character used for indentation (debug: space).
pub trait SyntaxInternerDisplay {
    /// Formats the value with a given indent level and an interner for resolving interned identifiers.
    ///
    /// # Arguments
    ///
    /// * `f` - the standard Rust formatter
    /// * `interner` - the interner used to resolve interned strings
    /// * `indent` - the indentation level (number of levels)
    ///
    /// # Example
    ///
    /// ```ignore
    /// my_value.fmt_syntax_with_indent(f, &interner, 2)?;
    /// ```
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result;

    /// Formats the value with zero indentation.
    ///
    /// Convenience method equivalent to `fmt_syntax_with_indent(f, interner, 0)`.
    fn fmt_syntax_with_interner(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        self.fmt_syntax_with_interner_and_indent(f, interner, 0)
    }

    /// Attempts to format the value into a `String` with a given indent level.
    ///
    /// Returns an error if formatting fails.
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    /// * `indent` - the indentation level
    ///
    /// # Returns
    ///
    /// A `Result` containing the formatted `String` or a formatting error.
    fn try_to_syntax_string_with_interner_and_indent(
        &self,
        interner: &SymbolInterner,
        indent: usize,
    ) -> Result<String, fmt::Error>
    where
        Self: Sized,
    {
        let mut s = String::new();
        write!(
            &mut s,
            "{}",
            DisplaySyntaxWrapper {
                value: self,
                interner,
                indent,
            }
        )?;
        Ok(s)
    }

    /// Formats the value into a `String` with a given indent level.
    ///
    /// Panics if formatting fails.
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    /// * `indent` - the indentation level
    ///
    /// # Returns
    ///
    /// The formatted `String`.
    fn to_syntax_string_with_interner_and_indent(
        &self,
        interner: &SymbolInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_syntax_string_with_interner_and_indent(interner, indent)
            .expect("Formatting into syntax string failed")
    }

    /// Attempts to format the value into a `String` with zero indentation.
    ///
    /// Returns an error if formatting fails.
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    ///
    /// # Returns
    ///
    /// A `Result` containing the formatted `String` or a formatting error.
    fn try_to_syntax_string_with_interner(
        &self,
        interner: &SymbolInterner,
    ) -> Result<String, std::fmt::Error>
    where
        Self: Sized,
    {
        self.try_to_syntax_string_with_interner_and_indent(interner, 0)
    }

    /// Formats the value into a `String` with zero indentation.
    ///
    /// Panics if formatting fails.
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    ///
    /// # Returns
    ///
    /// The formatted `String`.
    fn to_syntax_string_with_interner(&self, interner: &SymbolInterner) -> String
    where
        Self: Sized,
    {
        self.to_syntax_string_with_interner_and_indent(interner, 0)
    }
}

/// Internal wrapper used to implement [`std::fmt::Display`] by delegating to [`SyntaxInternerDisplay`].
///
/// This wrapper is private to the crate and intended for internal use.
pub(crate) struct DisplaySyntaxWrapper<'a, T: ?Sized> {
    /// Reference to the value to display.
    pub value: &'a T,

    /// Reference to the interner used to resolve identifiers.
    pub interner: &'a SymbolInterner,

    /// Indentation level to apply.
    pub indent: usize,
}

impl<'a, T: SyntaxInternerDisplay + ?Sized> std::fmt::Display for DisplaySyntaxWrapper<'a, T> {
    /// Formats the wrapped value by delegating to its [`SyntaxInternerDisplay::fmt_syntax_with_interner_and_indent`] implementation.
    ///
    /// This method is called when using the standard Rust formatting macros (e.g., `format!`, `println!`)
    /// on a `DisplaySyntaxWrapper`. It forwards the formatting request to the inner value,
    /// passing the stored interner and indentation level.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure of the formatting operation.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.value
            .fmt_syntax_with_interner_and_indent(f, self.interner, self.indent)
    }
}

/// Trait for formatting a value as a syntax string, without requiring an interner
/// or indentation.
///
/// This trait is intended for types where a simple, direct string representation
/// is needed for debugging, serialization, or output, without relying on external
/// resources such as a `StringInterner` or indentation management.
///
/// # Example
///
/// ```rust
/// use std::fmt;
///
/// struct MyType {
///     name: String,
/// }
///
/// impl SyntaxDisplay for MyType {
///     fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "MyType({})", self.name)
///     }
/// }
///
/// let value = MyType { name: "example".to_string() };
/// let s = value.to_syntax_string();
/// assert_eq!(s, "MyType(example)");
/// ```
pub trait SyntaxDisplay {
    /// Writes the value to the given formatter.
    ///
    /// This method is the common formatting function and is used by the debug
    /// implementations of `to_syntax_string` and `try_to_syntax_string`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the syntax string into.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Returns the value as a `String`.
    ///
    /// This is a convenience method that panics if formatting fails.
    ///
    /// # Panics
    ///
    /// Panics if writing to the internal string fails.
    fn to_syntax_string(&self) -> String
    where
        Self: Sized,
    {
        self.try_to_syntax_string()
            .expect("Failed to render syntax string")
    }

    /// Attempts to return the value as a `String`.
    ///
    /// Returns a `Result` containing the formatted string or a formatting error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::fmt;
    /// # use your_crate::SyntaxDisplay;
    /// # struct MyType;
    /// # impl SyntaxDisplay for MyType { fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "ok") } }
    /// let value = MyType;
    /// let s = value.try_to_syntax_string().unwrap();
    /// assert_eq!(s, "ok");
    /// ```
    fn try_to_syntax_string(&self) -> Result<String, fmt::Error>
    where
        Self: Sized,
    {
        let mut output = String::new();
        write!(&mut output, "{}", DisplayWrapper { value: self })?;
        Ok(output)
    }
}

/// Internal wrapper to allow using `fmt_syntax` with `write!`.
///
/// This struct is used by the debug implementations of
/// `try_to_syntax_string` and `to_syntax_string` to adapt a
/// `SyntaxDisplay` into a typing that implements `Display`.
struct DisplayWrapper<'a, T: ?Sized> {
    value: &'a T,
}

impl<'a, T: SyntaxDisplay + ?Sized> fmt::Display for DisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_syntax(f)
    }
}
