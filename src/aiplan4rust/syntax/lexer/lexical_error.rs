//! This module defines the `LexicalError` enum, representing errors that may occur during lexical analysis
//! and token parsing phases.
//!
//! The enum categorizes common lexical errors such as:
//! - Invalid tokens encountered in the input stream.
//! - Failures when parsing floating-point numbers (`ParseFloatError`).
//!
//! It provides conversion from `ParseFloatError` into `LexicalError` for seamless error handling,
//! and implements the `fmt::Display` trait to format errors as human-readable messages suitable
//! for debugging or user feedback.
//!
//! This error type is essential for robust lexers and parsers, enabling clear distinction and reporting
//! of lexical issues encountered while processing input source code.

use std::fmt;
use std::num::ParseFloatError;

/// Enum representing lexical errors that can occur during token parsing.
///
/// This enum is used to categorize different types of lexical errors encountered
/// during the lexical analysis phase, such as invalid tokens or parsing errors
/// for floating-point values.
///
#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    /// Represents an invalid token encountered during parsing.
    #[default]
    InvalidToken,
    /// Represents a parsing error that occurred while attempting to parse a floating-point value.
    InvalidNumber(ParseFloatError),
}

/// Implementation of the `From<ParseFloatError>` trait for converting
/// a `ParseFloatError` into a `LexicalError`.
///
/// This implementation allows for the conversion of a floating-point parsing
/// error (`ParseFloatError`) into a lexical error of type `LexicalError::InvalidFloat`.
impl From<ParseFloatError> for LexicalError {
    fn from(err: ParseFloatError) -> Self {
        LexicalError::InvalidNumber(err)
    }
}

/// Implements the `fmt::Display` trait for `LexicalError`.
///
/// This trait implementation allows `LexicalError` to be formatted as a human-readable string.
/// It defines how errors of type `LexicalError` should be displayed when printed using the
/// `format!` macro or when the `println!` macro is used. This provides more context for debugging
/// or logging lexical errors.
///
/// The implementation works as follows:
/// - If the error is `InvalidToken`, it will be displayed as `"Invalid Token"`.
/// - If the error is `InvalidFloat`, it will display the message from the underlying `
///   ParseFloatError` like: `"Invalid Float: <error message>"`.
impl fmt::Display for LexicalError {
    /// Formats the `LexicalError` into a user-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter that will format the error.
    ///
    /// # Returns
    ///
    /// The result of writing the formatted string to `f`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexicalError::InvalidToken => write!(f, "Invalid token"),
            LexicalError::InvalidNumber(err) => write!(f, "Invalid number: {}", err),
        }
    }
}
