//! Errors related to planning source detection and analysis.

use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::syntax::SyntaxError;

/// Enum representing all possible errors when constructing or analyzing a `SourceDescriptor`.
#[derive(Debug, Error)]
pub enum SourceError {
    /// Propagate errors from the serialization subsystem.
    #[error(transparent)]
    Serialization(#[from] SerializationError),

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

    /// Cannot determine the role (domain/problem) of the source.
    #[error("Cannot determine role (Domain/Problem) for source `{path}`")]
    UnknownRole { path: String },

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

impl SourceError {
    /// Create a new IO error variant.
    ///
    /// Converts the given path to a String (handling non-UTF8 paths gracefully)
    /// and wraps the original IO error.
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        SourceError::Io {
            path: path.as_ref().to_string_lossy().to_string(),
            source,
        }
    }

    /// Create a new InvalidHeader error from a path string.
    pub fn invalid_header(path: impl AsRef<Path>) -> Self {
        SourceError::InvalidHeader {
            path: path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Create a new UnknownRole error from a path.
    ///
    /// Use this when the role of a raw source cannot be determined.
    pub fn unknown_role(path: impl AsRef<Path>) -> Self {
        SourceError::UnknownRole {
            path: path.as_ref().to_string_lossy().to_string(),
        }
    }
    /// Helper to create an error for an unexpected raw source.
    pub fn unexpected_raw_source() -> Self {
        SourceError::UnexpectedRawSource
    }

    /// Helper to create an error for an unexpected serialized source.
    pub fn unexpected_serialized_source() -> Self {
        SourceError::UnexpectedSerializedSource
    }

    /// Helper to create an error for unknown source format.
    pub fn unknown_source() -> Self {
        SourceError::UnknownSource
    }

    pub fn open_failed(path: PathBuf, err: std::io::Error) -> Self {
        SourceError::OpenFailed { path, source: err }
    }

    pub fn read_failed(path: PathBuf, err: std::io::Error) -> Self {
        SourceError::ReadFailed { path, source: err }
    }
}
