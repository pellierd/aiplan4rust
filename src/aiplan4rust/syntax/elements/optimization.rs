use crate::aiplan4rust::syntax::lexer::token::MAXIMIZE;
use crate::aiplan4rust::syntax::lexer::token::MINIMIZE;
use crate::aiplan4rust::syntax::SyntaxDisplay;

use serde::Deserialize;
use serde::Serialize;

use std::fmt;

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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl SyntaxDisplay for Optimization {}
