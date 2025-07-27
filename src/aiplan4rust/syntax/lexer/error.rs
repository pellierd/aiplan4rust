//! Defines the `LexicalError` enum for errors occurring during lexical analysis and token parsing.
//!
//! This error type_checker covers invalid tokens and parsing failures for floating-point numbers (`ParseFloatError`).
//! It supports automatic conversion from `ParseFloatError` and derives human-readable error messages
//! via the `thiserror` crate.

use thiserror::Error;
use std::num::ParseFloatError;

/// Errors that may arise during lexical analysis.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LexicalError {
    /// An invalid token was encountered.
    #[error("Invalid token")]
    InvalidToken,

    /// Failed to parse a floating-point number.
    #[error("Invalid number: {0}")]
    InvalidNumber(#[from] ParseFloatError),
}

impl Default for LexicalError {
    fn default() -> Self {
        LexicalError::InvalidToken
    }
}
