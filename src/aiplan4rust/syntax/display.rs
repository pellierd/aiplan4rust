//! Module `syntax_display`
//!
//! This module defines the `SyntaxDisplay` trait for formatting structures
//! with configurable indentation and identifier resolution via a `StringInterner`.
//!
//! It allows formatting a type_checker into a string considering indentation level
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

/// Default number of characters used per indentation level.
///
/// This constant is used by [`SyntaxDisplay`] implementations as the default
/// indent width, typically 2 spaces.
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Default character used for indentation.
///
/// This constant is used by [`SyntaxDisplay`] implementations as the default
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
    /// ```rust
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
    ///
    /// Convenience method equivalent to `fmt_syntax_with_indent(f, interner, 0)`.
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
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    /// * `indent` - the indentation level
    ///
    /// # Returns
    ///
    /// A `Result` containing the formatted `String` or a formatting error.
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
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    /// * `indent` - the indentation level
    ///
    /// # Returns
    ///
    /// The formatted `String`.
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
    ///
    /// # Arguments
    ///
    /// * `interner` - the interner used to resolve interned strings
    ///
    /// # Returns
    ///
    /// The formatted `String`.
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
    /// Formats the wrapped value by delegating to its [`SyntaxDisplay::fmt_syntax_with_indent`] implementation.
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
    /// A [`std::fmt::Result`] indicating success or failure of the formatting operation.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_syntax_with_indent(f, self.interner, self.indent)
    }
}
