//! Errors related to serialization and deserialization processes.
//!
//! This module defines the [`SerializationError`] enum, used to report issues encountered
//! when working with different serialization formats such as JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! It includes errors for unsupported file formats, base64 decoding, file I/O, and serialization ops.

use thiserror::Error;

/// Represents all possible errors that may occur during serialization or deserialization operations.
#[derive(Debug, Error)]
pub enum SerializationError {

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Serde YAML error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error("Serde TOML error: {0}")]
    SerdeToml(#[from] toml::de::Error),

    #[error("Serde TOML serialization error: {0}")]
    SerdeTomlSer(#[from] toml::ser::Error),

    #[error("CBOR deserialization error: {0}")]
    Cbor(#[from] serde_cbor::Error),

    #[error("MessagePack deserialization error: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),

    #[error("MessagePack serialization error: {0}")]
    MessagePackSer(#[from] rmp_serde::encode::Error),

    /// Returned when a file extension is not supported.
    #[error("Unsupported extension: {0}")]
    UnsupportedExtensionError(String),

    /// Returned when a file has no extension.
    #[error("Missing file extension")]
    MissingExtensionError,

    /// Returned when the serialization format string is unsupported.
    #[error("Unsupported syntax format: {0}")]
    UnsupportedFormat(String),

    /// Returned when reading a file fails.
    #[error("Failed to read file: {0}")]
    FileReadError(String),

    /// Returned when writing to a file fails.
    #[error("Failed to write file: {0}")]
    FileWriteError(String),

    /// Indicates that the magic number in the header is invalid.
    ///
    /// This usually means that the file was not produced by the application
    /// or is corrupted.
    #[error("Invalid magic number in file header")]
    InvalidMagic,

    /// Error returned when a serialized string or file does not contain a valid header.
    #[error("Invalid or missing header in serialized data")]
    InvalidHeader,

    /// Returned when the type of the serialized object is not supported.
    #[error("Unsupported object type for serialization")]
    SerializationUnsupportedType,
}

impl SerializationError {
    /// Constructs an `UnsupportedExtensionError`.
    pub fn unsupported_extension<S: Into<String>>(ext: S) -> Self {
        Self::UnsupportedExtensionError(ext.into())
    }

    /// Constructs a `MissingExtensionError`.
    pub fn missing_extension() -> Self {
        Self::MissingExtensionError
    }

    /// Constructs an `UnsupportedFormat` error.
    pub fn unsupported_format<S: Into<String>>(fmt: S) -> Self {
        Self::UnsupportedFormat(fmt.into())
    }

    /// Constructs a `FileReadError`.
    ///
    /// # Arguments
    /// * `msg` - The error message describing why the file could not be read.
    pub fn file_read<S: Into<String>>(msg: S) -> Self {
        Self::FileReadError(msg.into())
    }

    /// Constructs a `FileWriteError`.
    ///
    /// # Arguments
    /// * `msg` - The error message describing why the file could not be written.
    pub fn file_write<S: Into<String>>(msg: S) -> Self {
        Self::FileWriteError(msg.into())
    }

    /// Constructs an `InvalidMagic` error.
    pub fn invalid_magic() -> Self {
        Self::InvalidMagic
    }

    /// Constructs an `InvalidHeader` error.
    ///
    /// This error occurs when a serialized string or file does not contain a valid header,
    /// for example if the header cannot be separated from the payload using
    /// `HEADER_PAYLOAD_SEPARATOR` or is malformed JSON.
    pub fn invalid_header() -> Self {
        Self::InvalidHeader
    }

    /// Constructs a `SerializationUnsupportedType` error.
    ///
    /// This error occurs when attempting to serialize an object whose type
    /// is not supported by the serialization system, e.g., a type not listed
    /// in `SerdeSerializableType`.
    pub fn unsupported_serialization_type() -> Self {
        Self::SerializationUnsupportedType
    }
}
