//! Errors related to planning source detection and artefact management.
//!
//! This module defines the `ArtefactError` enum, which captures all possible
//! errors that can occur when constructing, reading, or analyzing a
//! `SourceDescriptor` or its associated artefacts.
//!
//! The errors cover several categories:
//! - **JSON serialization/deserialization** via Serde (`SerdeJson`).
//! - **File I/O operations** (`IO`).
//! - **Internal serialization subsystem** errors (`Serialization`).
//! - **Invalid access** to specific content types from a `Source` (`MissingRawContent`,
//!   `MissingIRContent`, `MissingBinaryContent`, `MissingTextContent`).
//! - **Invalid file paths or names** (`InvalidFileName`).
//!
//! Each variant either propagates the original error or provides a descriptive
//! message to aid debugging.
//!
//! # Example
//! ```rust,ignore
//! use aiplan4rust::artefact::ArtefactError;
//!
//! fn example() -> Result<(), ArtefactError> {
//!     // Simulate an operation that may fail
//!     Err(ArtefactError::MissingRawContent)
//! }
//! ```
use crate::aiplan4rust::cli::io::serialization::SerializationError;
use thiserror::Error;

/// Represents all possible errors when constructing, reading, or analyzing a `SourceDescriptor` or its artefacts.
#[derive(Debug, Error)]
pub enum ArtefactError {
    /// Error originating from JSON (de)serialization via Serde.
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// Generic I/O error.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    /// Propagates errors from the internal serialization subsystem.
    #[error(transparent)]
    Serialization(#[from] SerializationError),

    /// Attempted to access `RawContent` from a source that is not raw.
    #[error("Cannot get RawContent: Source is not a raw source")]
    MissingRawContent,

    /// Attempted to access `IRContent` from a source that is not IR.
    #[error("Cannot get IRContent: Source is not an IR source")]
    MissingIRContent,

    /// Attempted to access unknown binary content from a source that is not binary.
    #[error("Cannot get unknown binary content: Source is not a binary unknown source")]
    MissingBinaryContent,

    /// Attempted to access unknown text content from a source that is not text.
    #[error("Cannot get unknown text content: Source is not a text unknown source")]
    MissingTextContent,
}

impl ArtefactError {
    /// Constructs an error indicating that a `Source` expected to be raw
    /// does not contain `RawContent`.
    ///
    /// # Example
    /// ```rust,ignore
    /// return Err(ArtefactError::missing_raw_content());
    /// ```
    pub fn missing_raw_content() -> Self {
        ArtefactError::MissingRawContent
    }

    /// Constructs an error indicating that a `Source` expected to be IR
    /// does not contain `IRContent`.
    ///
    /// # Example
    /// ```rust,ignore
    /// return Err(ArtefactError::missing_ir_content());
    /// ```
    pub fn missing_ir_content() -> Self {
        ArtefactError::MissingIRContent
    }

    /// Constructs an error indicating that a `Source` expected to contain
    /// unknown binary content does not.
    ///
    /// # Example
    /// ```rust,ignore
    /// return Err(ArtefactError::missing_binary_content());
    /// ```
    pub fn missing_binary_content() -> Self {
        ArtefactError::MissingBinaryContent
    }

    /// Constructs an error indicating that a `Source` expected to contain
    /// unknown text content does not.
    ///
    /// # Example
    /// ```rust,ignore
    /// return Err(ArtefactError::missing_text_content());
    /// ```
    pub fn missing_text_content() -> Self {
        ArtefactError::MissingTextContent
    }
}
