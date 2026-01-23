//! Defines the `CustomParseError` enum for recoverable parser errors.
//!
//! This enum represents errors encountered during parsing of PDDL/HDDL,
//! including duplicate definitions, invalid section ordering, number parsing failures,
//! and generic errors with messages. All variants carry span information for diagnostics.

use thiserror::Error;
use crate::aiplan4rust::syntax::ast::AstKind;

/// A recoverable error in parsing, carrying source span information.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CustomParseError {

    /// Generic error with message and span.
    #[error("{0}")]
    Generic(String, usize, usize),

    /// Duplicate section or definition.
    #[error("Duplicate definition block: {0}")]
    DuplicateDefinitionBlock(AstKind, usize, usize),

    /// Section defined in an invalid order.
    #[error("Invalid definition block order: {0}")]
    InvalidDefinitionBlockOrder(AstKind, usize, usize, Vec<AstKind>),

    /// Failed to parse a floating-point number.
    #[error("Invalid number: {0}")]
    InvalidNumber(String, usize, usize),
}

impl Default for CustomParseError {
    fn default() -> Self {
        CustomParseError::Generic("unknown error".to_string(), 0, 0)
    }
}

impl CustomParseError {
    /// Creates a generic error with a custom message and span.
    ///
    /// # Arguments
    /// - `message`: A descriptive error message.
    /// - `start`: Byte offset where the error starts.
    /// - `end`: Byte offset where the error ends.
    ///
    /// # Returns
    /// - `CustomParseError::Default` instance.
    pub fn generic(message: impl Into<String>, start: usize, end: usize) -> Self {
        CustomParseError::Generic(message.into(), start, end)
    }

    /// Creates a duplicate definition block error.
    ///
    /// # Arguments
    /// - `def`: The `AstKind` of the duplicated block.
    /// - `start`: Byte offset where the duplicate starts.
    /// - `end`: Byte offset where the duplicate ends.
    ///
    /// # Returns
    /// - `CustomParseError::DuplicateDefinitionBlock` instance.
    pub fn duplicate_definition_block(def: AstKind, start: usize, end: usize) -> Self {
        CustomParseError::DuplicateDefinitionBlock(def, start, end)
    }

    /// Creates an invalid block order error.
    ///
    /// # Arguments
    /// - `def`: The `AstKind` of the misplaced section.
    /// - `start`: Byte offset where the block starts.
    /// - `end`: Byte offset where the block ends.
    /// - `ordered_def`: Vector of `AstKind`s representing the expected order.
    ///
    /// # Returns
    /// - `CustomParseError::InvalidDefinitionBlockOrder` instance.
    pub fn invalid_definition_block_order(def: AstKind, start: usize, end: usize, ordered_def: Vec<AstKind>) -> Self {
        CustomParseError::InvalidDefinitionBlockOrder(def, start, end, ordered_def)
    }

    /// Creates a number parsing error.
    ///
    /// # Arguments
    /// - `slice`: The string slice of the token that failed to parse as a number.
    /// - `start`: Byte offset where the invalid number starts.
    /// - `end`: Byte offset where the invalid number ends.
    ///
    /// # Returns
    /// - A `CustomParseError::InvalidNumber` instance representing the invalid numeric token.
    pub fn invalid_number(slice: String, start: usize, end: usize) -> Self {
        CustomParseError::InvalidNumber(slice, start, end)
    }
}
