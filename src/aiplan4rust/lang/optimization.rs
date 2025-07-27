//! Module defining the `Optimization` enum used to represent optimization goals
//! in a planning or constraint-solving context.
//!
//! This module provides a simple abstraction over optimization directives commonly
//! used in domain-specific languages or IRs (Intermediate Representations). Specifically,
//! it supports three states:
//! - `None`: No optimization is specified.
//! - `Minimize`: Indicates that the system should minimize a given objective.
//! - `Maximize`: Indicates that the system should maximize a given objective.
//!
//! The enum is designed to integrate easily with parsing (e.g., from PDDL keywords
//! like `:minimize` or `:maximize`), as well as serialization formats like JSON.
//!
//! It derives common utility traits such as `Clone`, `Copy`, `Debug`, `Eq`, `Hash`,
//! and also supports serialization through Serde.
//!
//! This type_checker is typically used in goal representations, metric definitions, or solver configurations.

use crate::aiplan4rust::syntax::lexer::token::MAXIMIZE;
use crate::aiplan4rust::syntax::lexer::token::MINIMIZE;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// This enum represents the two possible types of optimization.
///
/// It derives several traits:
/// - `Clone`: Allows cloning of enum instances.
/// - `Debug`: Enables formatted output for debugging purposes.
/// - `PartialEq` and `Eq`: Allows comparison for equality between enum instances.
/// - `Hash`: Enables the calculation of a hash value for enum instances, useful in data structures
///     like `HashMap` or `HashSet`.
/// - `Serialize` and `Deserialize`: Allow the enum to be serialized and deserialized, facilitating
///     storage or transmission in formats like JSON.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Optimization {
    /// No optimization specified (default value).
    #[default]
    None,

    /// Seeks to minimize the objective function.
    Minimize,

    /// Seeks to maximize the objective function.
    Maximize,
}

/// Implements the `fmt::Display` trait for the `Optimization` enum.
///
/// This allows an `Optimization` value to be formatted as a user-friendly string,
/// suitable for printing or logging.
///
/// # Arguments
///
/// * `f` - A mutable reference to a `fmt::Formatter` used to write the output.
///
/// # Returns
///
/// A `fmt::Result` indicating whether the formatting succeeded.
///
/// # Examples
///
/// ```
/// let opt = Optimization::Minimize;
/// assert_eq!(format!("{}", opt), "minimize");
/// ```
impl fmt::Display for Optimization {
    /// Formats the `Optimization` as a string representation.
    ///
    /// Matches each variant to its corresponding string:
    /// - `Minimize` => "minimize"
    /// - `Maximize` => "maximize"
    /// - `None` => "NONE"
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Optimization::Minimize => write!(f, "{}", MINIMIZE),
            Optimization::Maximize => write!(f, "{}", MAXIMIZE),
            Optimization::None => write!(f, "{}", "NONE"),
        }
    }
}


/// Implements the `DisplayWithInterner` trait for `Optimization`.
///
/// This implementation formats an `Optimization` value by
/// delegating to its standard `Display` trait, since it does not
/// require interner-based symbol resolution.
///
/// The `interner` parameter is unused.
///
/// # Example
///
/// ```
/// let opt = Optimization::Enabled;
/// let s = opt.to_string_with_interner(&interner);
/// assert_eq!(s, "Enabled");
/// ```
impl InternerDisplay for Optimization {
    /// Formats the `Optimization` using the given formatter and interner.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - The string interner (unused).
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}


/// Implements the `PlanningSyntaxDisplay` trait for `Optimization`.
///
/// This trait provides user-facing syntax formatting for `Optimization` values.
///
/// The implementation simply delegates to the standard `Display` trait,
/// ensuring consistent output across both traits.
///
/// The `interner` parameter is unused in this implementation.
///
/// # Example
///
/// ```
/// let opt = Optimization::Enabled;
/// let s = opt.to_string_syntax(&interner);
/// assert_eq!(s, "Enabled");
/// ```
impl SyntaxDisplay for Optimization {
    /// Formats the `Optimization` for planning syntax display.
    ///
    /// Delegates to the `Display` trait implementation.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter to write output to.
    /// * `_interner` - Unused interner parameter.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, _interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        fmt::Display::fmt(self, f)
    }
}
