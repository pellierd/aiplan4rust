use crate::aiplan4rust::syntax::lexer::token::MAXIMIZE;
use crate::aiplan4rust::syntax::lexer::token::MINIMIZE;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Optimization {
    /// Seeks to minimize the objective function.
    Minimize,
    /// Seeks to maximize the objective function.
    Maximize,
}

impl fmt::Display for Optimization {
    /// This method implements the `fmt::Display` trait for the `Optimization` enum.
    /// It allows an instance of `Optimization` to be formatted as a string for printing or logging
    /// purposes.
    ///
    /// # Arguments
    /// - `f`: A mutable reference to a `fmt::Formatter` that will be used to format the output
    ///     string.
    ///
    /// # Returns
    /// - Returns a `fmt::Result`, which indicates whether the formatting operation was successful.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Optimization::Minimize => write!(f, "{}", MINIMIZE),
            Optimization::Maximize => write!(f, "{}", MAXIMIZE),
        }
    }
}
/// Implements the `DisplayWithInterner` trait for `Optimization`.
///
/// This implementation formats the `Optimization` value by
/// delegating to the standard `Display` trait, as it doesn't
/// require interner-based resolution.
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
impl DisplayWithInterner for Optimization {
    fn fmt_with(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `DisplaySyntax` trait for `Optimization`.
///
/// This trait formats the value for user-facing syntax display.
///
/// By default, it calls `DisplayWithInterner::fmt_with`,
/// providing consistent formatting across both traits.
///
/// # Example
///
/// ```
/// let opt = Optimization::Enabled;
/// let s = opt.to_string_syntax(&interner);
/// assert_eq!(s, "Enabled");
/// ```
impl DisplaySyntax for Optimization {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
