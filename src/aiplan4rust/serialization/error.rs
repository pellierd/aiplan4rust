//! Errors related to serialization and deserialization processes.
//!
//! This module defines the [`SerializationError`] enum, used to report issues encountered
//! when working with different serialization formats such as JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! It includes errors for unsupported file formats, base64 decoding, file I/O, and serialization logic.

use thiserror::Error;

/// Represents all possible errors that may occur during serialization or deserialization operations.
#[derive(Debug, Error)]
pub enum SerializationError {
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

    /// Returned when general deserialization fails.
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Returned when base64 decoding fails.
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(String),

    /// JSON-specific deserialization failure.
    #[error("JSON deserialization error: {0}")]
    JsonDeserializationError(String),

    /// YAML-specific deserialization failure.
    #[error("YAML deserialization error: {0}")]
    YamlDeserializationError(String),

    /// TOML-specific deserialization failure.
    #[error("TOML deserialization error: {0}")]
    TomlDeserializationError(String),

    /// CBOR-specific deserialization failure.
    #[error("CBOR deserialization error: {0}")]
    CborDeserializationError(String),

    /// MessagePack-specific deserialization failure.
    #[error("MessagePack deserialization error: {0}")]
    MessagePackDeserializationError(String),

    /// JSON-specific serialization failure.
    #[error("JSON serialization error: {0}")]
    JsonSerializationError(String),

    /// YAML-specific serialization failure.
    #[error("YAML serialization error: {0}")]
    YamlSerializationError(String),

    /// TOML-specific serialization failure.
    #[error("TOML serialization error: {0}")]
    TomlSerializationError(String),

    /// CBOR-specific serialization failure.
    #[error("CBOR serialization error: {0}")]
    CborSerializationError(String),

    /// MessagePack-specific serialization failure.
    #[error("MessagePack serialization error: {0}")]
    MessagePackSerializationError(String),
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

    /// Constructs a general `DeserializationError`.
    pub fn deserialization<S: Into<String>>(msg: S) -> Self {
        Self::DeserializationError(msg.into())
    }

    /// Constructs a `Base64DecodeError`.
    pub fn base64_decode<S: Into<String>>(msg: S) -> Self {
        Self::Base64DecodeError(msg.into())
    }

    /// Constructs a `JsonDeserializationError`.
    pub fn json_deserialization<S: Into<String>>(msg: S) -> Self {
        Self::JsonDeserializationError(msg.into())
    }

    /// Constructs a `YamlDeserializationError`.
    pub fn yaml_deserialization<S: Into<String>>(msg: S) -> Self {
        Self::YamlDeserializationError(msg.into())
    }

    /// Constructs a `TomlDeserializationError`.
    pub fn toml_deserialization<S: Into<String>>(msg: S) -> Self {
        Self::TomlDeserializationError(msg.into())
    }

    /// Constructs a `CborDeserializationError`.
    pub fn cbor_deserialization<S: Into<String>>(msg: S) -> Self {
        Self::CborDeserializationError(msg.into())
    }

    /// Constructs a `MessagePackDeserializationError`.
    pub fn messagepack_deserialization<S: Into<String>>(msg: S) -> Self {
        Self::MessagePackDeserializationError(msg.into())
    }

    /// Constructs a `JsonSerializationError`.
    pub fn json_serialization<S: Into<String>>(msg: S) -> Self {
        Self::JsonSerializationError(msg.into())
    }

    /// Constructs a `YamlSerializationError`.
    pub fn yaml_serialization<S: Into<String>>(msg: S) -> Self {
        Self::YamlSerializationError(msg.into())
    }

    /// Constructs a `TomlSerializationError`.
    pub fn toml_serialization<S: Into<String>>(msg: S) -> Self {
        Self::TomlSerializationError(msg.into())
    }

    /// Constructs a `CborSerializationError`.
    pub fn cbor_serialization<S: Into<String>>(msg: S) -> Self {
        Self::CborSerializationError(msg.into())
    }

    /// Constructs a `MessagePackSerializationError`.
    pub fn messagepack_serialization<S: Into<String>>(msg: S) -> Self {
        Self::MessagePackSerializationError(msg.into())
    }
}
