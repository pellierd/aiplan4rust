//! Module `syntax_display`
//!
//! This module defines the `SyntaxDisplay` trait for formatting structures
//! with configurable indentation and identifier resolution via a `StringInterner`.
//!
//! It allows formatting a type into a string considering indentation level
//! and an interner to resolve interned identifiers.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::interner::StringInterner;
//! use std::fmt;
//! struct MyType {
//!     id: usize,
//! }
//!
//! impl SyntaxDisplay for MyType {
//!     fn fmt_syntax_with_indent(
//!         &self,
//!         f: &mut fmt::Formatter<'_>,
//!         interner: &StringInterner,
//!         indent: usize,
//!     ) -> fmt::Result {
//!         let indent_str = Self::make_indent(indent);
//!         write!(f, "{}{}", indent_str, interner.resolve(self.id))
//!     }
//! }
//! ```

use std::fmt::Write;
use crate::aiplan4rust::interner::StringInterner;

/// Trait for formatting values with planned syntax, supporting indentation and interner resolution.
///
/// Provides default constants for indentation width and indent character,
/// along with methods to format the value at variable indentation levels.
///
/// # Default constants
///
/// - [`DEFAULT_INDENT_WIDTH`]: number of characters per indent level (default: 2).
/// - [`DEFAULT_INDENT_CHAR`]: character used for indentation (default: space).
pub trait SyntaxDisplay {
    /// Default indentation width per level.
    const DEFAULT_INDENT_WIDTH: usize = 2;

    /// Default indentation character.
    const DEFAULT_INDENT_CHAR: char = ' ';

    /// Returns the indentation width to use.
    ///
    /// Can be overridden to customize indentation width.
    fn indent_width(&self) -> usize {
        Self::DEFAULT_INDENT_WIDTH
    }

    /// Returns the indentation character to use.
    ///
    /// Can be overridden to customize indent character.
    fn indent_char(&self) -> char {
        Self::DEFAULT_INDENT_CHAR
    }

    /// Generates an indentation string for a given level.
    ///
    /// The string contains `indent_width * level` occurrences of `indent_char`.
    ///
    /// # Example
    ///
    /// ```
    /// let indent = MyType::make_indent(3); // "      " (6 spaces if indent_width=2)
    /// ```
    fn make_indent(level: usize) -> String {
        let total = level * Self::DEFAULT_INDENT_WIDTH;
        std::iter::repeat(Self::DEFAULT_INDENT_CHAR)
            .take(total)
            .collect()
    }

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
    fn fmt_syntax_with_indent(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> std::fmt::Result;

    /// Formats the value with zero indentation.
    fn fmt_syntax(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        self.fmt_syntax_with_indent(f, interner, 0)
    }

    /// Attempts to format the value into a `String` with a given indent level.
    ///
    /// Returns an error if formatting fails.
    fn try_to_syntax_string_with_indent(
        &self,
        interner: &StringInterner,
        indent: usize,
    ) -> Result<String, std::fmt::Error>
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
    fn to_syntax_string_with_indent(
        &self,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_syntax_string_with_indent(interner, indent)
            .expect("Formatting into syntax string failed")
    }

    /// Attempts to format the value into a `String` with zero indentation.
    fn try_to_syntax_string(
        &self,
        interner: &StringInterner,
    ) -> Result<String, std::fmt::Error>
    where
        Self: Sized,
    {
        self.try_to_syntax_string_with_indent(interner, 0)
    }

    /// Formats the value into a `String` with zero indentation.
    ///
    /// Panics if formatting fails.
    fn to_syntax_string(
        &self,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.to_syntax_string_with_indent(interner, 0)
    }
}

/// Internal wrapper used to implement [`std::fmt::Display`] by delegating to [`SyntaxDisplay`].
///
/// This wrapper is private to the crate and intended for internal use.
pub(crate) struct DisplaySyntaxWrapper<'a, T: ?Sized> {
    /// Reference to the value to display.
    pub value: &'a T,

    /// Reference to the interner used to resolve identifiers.
    pub interner: &'a StringInterner,

    /// Indentation level to apply.
    pub indent: usize,
}

impl<'a, T: SyntaxDisplay + ?Sized> std::fmt::Display for DisplaySyntaxWrapper<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_syntax_with_indent(f, self.interner, self.indent)
    }
}
