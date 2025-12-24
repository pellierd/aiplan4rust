//! Errors related to planning source detection and analysis.

use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::aiplan4rust::serialization::SerializationError;

/// Enum representing all possible errors when constructing or analyzing a `SourceDescriptor`.
#[derive(Debug, Error)]
pub enum IOError {

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    /// Propagate errors from the serialization subsystem.
    #[error(transparent)]
    Serialization(#[from] SerializationError),

    /// Error when trying to access RawContent from an Input that is not Raw.
    #[error("Cannot get RawContent: Input is not a raw source")]
    MissingRawContent,

    /// Error when trying to access IRContent from an Input that is not IR.
    #[error("Cannot get IRContent: Input is not an IR source")]
    MissingIRContent,

    /// Error when trying to access unknown binary content from an Input that is not BinaryUnknown.
    #[error("Cannot get unknown binary content: Input is not a binary unknown source")]
    MissingUnknownBinaryContent,

    /// Error when trying to access unknown text content from an Input that is not UnknownText.
    #[error("Cannot get unknown text content: Input is not a text unknown source")]
    MissingUnknownTextContent,

    /// Cannot extract a valid file stem from the given path.
    ///
    /// This usually happens when the path does not contain a valid filename
    /// or when the filename is not valid UTF-8.
    #[error("Invalid file name: '{0}'")]
    InvalidFileName(String),

    /// Error reading the file from disk.
    #[error("Failed to read file `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Header in a serialized file is invalid or not supported.
    #[error("Invalid serialized header in file `{path}`")]
    InvalidHeader { path: String },

    /// Error when an unexpected raw source is encountered where a serialized source was expected.
    #[error("Unexpected raw source: cannot treat as serialized")]
    UnexpectedRawSource,

    /// Error when an unexpected serialized source is encountered during parsing.
    #[error("Unexpected serialized source: cannot parse as raw PDDL/HDDL")]
    UnexpectedSerializedSource,

    /// Error when the source format is unknown and cannot be parsed.
    #[error("Cannot parse unknown source format")]
    UnknownSource,

    /// Failed to open the source file.
    #[error("Failed to open source file '{path}': {source}")]
    OpenFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to read the source file.
    #[error("Failed to read source file '{path}': {source}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl IOError {

    pub fn missing_raw_content() -> Self {
        IOError::MissingRawContent
    }

    pub fn missing_ir_content() -> Self {
        IOError::MissingIRContent
    }

    pub fn missing_unknown_binary_content() -> Self {
        IOError::MissingUnknownBinaryContent
    }

    pub fn missing_unknown_text_content() -> Self {
        IOError::MissingUnknownTextContent
    }


    /// Creates an error indicating that a file name is invalid.
    ///
    /// This is typically used when a file path does not contain a valid
    /// filename or when the filename is not valid UTF-8.
    pub fn invalid_file_name(path: impl Into<String>) -> Self {
        IOError::InvalidFileName(path.into())
    }

    /// Create a new IO error variant.
    ///
    /// Converts the given path to a String (handling non-UTF8 paths gracefully)
    /// and wraps the original IO error.
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        IOError::Io {
            path: path.as_ref().to_string_lossy().to_string(),
            source,
        }
    }

    /// Create a new InvalidHeader error from a path string.
    pub fn invalid_header(path: impl AsRef<Path>) -> Self {
        IOError::InvalidHeader {
            path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Helper to create an error for an unexpected raw source.
    pub fn unexpected_raw_source() -> Self {
        IOError::UnexpectedRawSource
    }

    /// Helper to create an error for an unexpected serialized source.
    pub fn unexpected_serialized_source() -> Self {
        IOError::UnexpectedSerializedSource
    }

    /// Helper to create an error for unknown source format.
    pub fn unknown_source() -> Self {
        IOError::UnknownSource
    }

    pub fn open_failed(path: PathBuf, err: std::io::Error) -> Self {
        IOError::OpenFailed { path, source: err }
    }

    pub fn read_failed(path: PathBuf, err: std::io::Error) -> Self {
        IOError::ReadFailed { path, source: err }
    }
}
