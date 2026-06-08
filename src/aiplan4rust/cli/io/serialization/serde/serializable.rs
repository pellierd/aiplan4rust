//! Module `serialization`
//!
//! This module provides the `Serializable` trait for serializing and deserializing
//! data structures using multiple Serde-supported formats.
//!
//! Supported formats include JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! The trait handles serialization to/from strings and files, with error handling
//! via `ParserInternalError`.

use crate::aiplan4rust::cli::io::serialization::serde::SerdeFormat;
use crate::aiplan4rust::cli::io::serialization::SerializationError;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for serializing and deserializing objects using Serde-supported formats.
///
/// This trait supports formats: JSON, YAML, TOML, CBOR, and MessagePack.
///
/// It provides methods to serialize/deserialize to/from strings and files,
/// returning errors wrapped in `SerializationError` on failure.
pub trait Serializable: Serialize + DeserializeOwned {
    /// Serializes the object into bytes in the specified format.
    ///
    /// This function supports both text-based formats (JSON, YAML, TOML) and binary formats
    /// (CBOR, MessagePack). It returns the serialized bytes directly without any envelope or header.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    ///
    /// # Returns
    ///
    /// Returns a `Vec<u8>` containing the serialized object, or a [`SerializationError`] if serialization fails.
    ///
    /// # Errors
    ///
    /// Propagates errors from the underlying serialization libraries:
    /// - [`SerializationError::SerdeJson`] for JSON
    /// - [`SerializationError::SerdeYaml`] for YAML
    /// - [`SerializationError::SerdeToml`] for TOML
    /// - [`SerializationError::Cbor`] for CBOR
    /// - [`SerializationError::MessagePack`] for MessagePack
    ///
    /// # Examples
    ///
    /// ```rust
    /// let json_bytes = obj.serialize_to_bytes(SerdeFormat::Json)?;
    /// let cbor_bytes = obj.serialize_to_bytes(SerdeFormat::Cbor)?;
    /// ```
    fn serialize_to_bytes(&self, format: SerdeFormat) -> Result<Vec<u8>, SerializationError> {
        let bytes = match format {
            SerdeFormat::Json => serde_json::to_string_pretty(self)?.into_bytes(),
            SerdeFormat::Yaml => serde_yaml::to_string(self)?.into_bytes(),
            SerdeFormat::Toml => toml::to_string(self)?.into_bytes(),
            SerdeFormat::Cbor => serde_cbor::to_vec(self)?,
            SerdeFormat::MessagePack => rmp_serde::to_vec(self)?,
        };

        Ok(bytes)
    }

    /// Serializes the object to a file in the specified format.
    ///
    /// This function serializes the object into the given `SerdeFormat` (JSON, YAML, TOML, CBOR,
    /// or MessagePack) and writes the resulting bytes to the specified file path.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    /// * `path` - The file path where the serialized data will be written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the object was successfully serialized and written to the file.
    /// Returns a [`SerializationError`] if serialization fails or if writing to the file fails.
    ///
    /// # Errors
    ///
    /// This function can return:
    /// - Any serialization error from the underlying format (`SerdeJson`, `SerdeYaml`, `SerdeToml`,
    ///   `Cbor`, `MessagePack`).
    /// - `SerializationError::Io` if writing to the file fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aiplan4rust::serialization::{SerdeFormat, Serializable};
    /// # let obj: MyType = /* ... */ ;
    /// obj.serialize_to_file(SerdeFormat::Json, "output.json")?;
    /// obj.serialize_to_file(SerdeFormat::Yaml, std::path::Path::new("config.yaml"))?;
    /// ```
    fn serialize_to_file<P: AsRef<std::path::Path>>(
        &self,
        format: SerdeFormat,
        path: P,
    ) -> Result<(), SerializationError> {
        let content = self.serialize_to_bytes(format)?;
        std::fs::write(path.as_ref(), content).map_err(SerializationError::from)?;
        Ok(())
    }

    /// Serializes the object (wrapped in a `SerdeEnvelope` with header) and writes it to a file,
    /// automatically inferring the serialization format from the file extension.
    ///
    /// # Arguments
    ///
    /// * `path` - The output file path. The file extension must be recognized
    ///            to determine the serialization format.
    ///
    /// # Supported formats and their extensions:
    ///
    /// | Format      | Recognized extensions       |
    /// |-------------|-----------------------------|
    /// | JSON        | `.json`                     |
    /// | YAML        | `.yaml`, `.yml`             |
    /// | TOML        | `.toml`                     |
    /// | CBOR        | `.cbor`                     |
    /// | MessagePack | `.msgpack`, `.messagepack`  |
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if:
    /// - The file extension is missing or unknown.
    /// - Serialization of the object fails.
    /// - Writing to the file fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let obj: MyType = /* ... */ ;
    /// obj.serialize_to_file_with_auto_format("output.json")?;
    /// obj.serialize_to_file_with_auto_format(PathBuf::from("config.yaml"))?;
    /// ```
    fn serialize_to_file_with_auto_format<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), SerializationError> {
        let path_ref = path.as_ref();

        // Infer format from file extension
        let format = format_from_path(path_ref)?;

        // Serialize and write
        self.serialize_to_file(format, path_ref.to_path_buf())
    }

    /// Deserializes an object from a byte slice using the specified serialization format.
    ///
    /// This function operates directly on raw bytes and supports both text-based formats
    /// (JSON, YAML, TOML) and binary formats (CBOR, MessagePack). It does **not** require
    /// any header or envelope structure—only the raw serialized data is expected.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A byte slice containing the serialized object.
    /// * `format` - The serialization format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    ///
    /// # Returns
    ///
    /// Returns the deserialized object of typing `T` on success, or a [`SerializationError`]
    /// if deserialization fails.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializationError`] if:
    /// - The byte slice cannot be interpreted as UTF-8 for text-based formats (JSON, YAML, TOML).
    /// - The deserialization fails for the specified format (e.g., malformed JSON, invalid CBOR, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aiplan4rust::serialization::{SerdeFormat, SerializationError};
    /// # use serde::Deserialize;
    /// #[derive(Deserialize)]
    /// struct MyType {
    ///     key: String,
    /// }
    ///
    /// let payload_bytes: &[u8] = br#"{"key":"value"}"#;
    /// let obj: MyType = deserialize_from_bytes(payload_bytes, SerdeFormat::Json)?;
    /// ```
    fn deserialize_from_bytes<T: DeserializeOwned>(
        bytes: &[u8],
        format: SerdeFormat,
    ) -> Result<T, SerializationError> {
        match format {
            SerdeFormat::Json => {
                let s = std::str::from_utf8(bytes)?;
                serde_json::from_str(s).map_err(SerializationError::from)
            }
            SerdeFormat::Yaml => {
                let s = std::str::from_utf8(bytes)?;
                serde_yaml::from_str(s).map_err(SerializationError::from)
            }
            SerdeFormat::Toml => {
                let s = std::str::from_utf8(bytes)?;
                toml::from_str(s).map_err(|e| SerializationError::SerdeToml(e))
            }
            SerdeFormat::Cbor => serde_cbor::from_slice(bytes).map_err(SerializationError::from),
            SerdeFormat::MessagePack => {
                rmp_serde::from_slice(bytes).map_err(SerializationError::from)
            }
        }
    }

    /// Deserializes an object from a file using the specified serialization format.
    ///
    /// This function reads the file at the given path as raw bytes, which allows
    /// supporting both text-based and binary serialization formats (JSON, YAML, TOML,
    /// CBOR, MessagePack). The provided `format` argument specifies how the payload
    /// should be deserialized.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file containing the serialized data.
    /// * `format` - The serialization format of the payload (`SerdeFormat`).
    ///
    /// # Returns
    ///
    /// Returns the deserialized object of typing `Self` on success, or a [`SerializationError`] if an error occurs.
    ///
    /// # Errors
    ///
    /// This function may return errors including, but not limited to:
    /// - [`SerializationError::FileReadError`] if the file cannot be read.
    /// - [`SerializationError::Utf8`] if the file contains invalid UTF-8 when required by the format.
    /// - [`SerializationError::SerdeJson`], [`SerializationError::SerdeYaml`], [`SerializationError::SerdeToml`],
    ///   [`SerializationError::Cbor`], or [`SerializationError::MessagePack`] if deserialization fails for the given format.
    ///
    /// # Example
    ///
    /// ```rust
    /// let obj: MyType = MyType::deserialize_from_file_with_format("data.json", SerdeFormat::Json)?;
    /// ```
    fn deserialize_from_file(path: &str, format: SerdeFormat) -> Result<Self, SerializationError>
    where
        Self: Sized + Serialize + DeserializeOwned,
    {
        // Read the file as bytes
        let bytes = std::fs::read(path)?; // IO errors are automatically propagated via #[from]

        // Deserialize directly from bytes using the provided format
        Self::deserialize_from_bytes(&bytes, format)
    }

    /// Deserializes an object from a file, automatically inferring the serialization format
    /// from the file extension while still validating the file header.
    ///
    /// The file is expected to start with a valid `SerdeHeader` (magic number, version,
    /// format, timestamp) serialized in JSON, followed by the payload. The header is always
    /// in JSON, but the payload format is inferred from the file extension.
    ///
    /// # Supported formats and recognized extensions
    ///
    /// | Format      | Extensions                  |
    /// |-------------|-----------------------------|
    /// | JSON        | `.json`                     |
    /// | YAML        | `.yaml`, `.yml`             |
    /// | TOML        | `.toml`                     |
    /// | CBOR        | `.cbor`                     |
    /// | MessagePack | `.msgpack`, `.messagepack`  |
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if:
    /// - The file extension is missing (`missing_extension` error variant).
    /// - The file extension is not recognized as a supported format (`unsupported_extension`).
    /// - The file cannot be read (`file_read` error variant).
    /// - The header cannot be parsed or has an invalid magic number (`InvalidMagic`).
    /// - The payload cannot be deserialized according to the inferred format.
    ///
    /// # Example
    ///
    /// ```rust
    /// let obj: MyType = MyType::deserialize_from_file_with_auto_format("config.yaml")?;
    /// ```
    fn deserialize_from_file_with_auto_format<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        let path_ref = path.as_ref();

        // Infer format from file extension
        let format = format_from_path(path_ref)?;

        // Read the file as bytes
        let bytes = std::fs::read(path_ref).map_err(SerializationError::from)?;

        // Deserialize using the inferred format
        Self::deserialize_from_bytes(&bytes, format)
    }
}

/// Infers the serialization `Format` from a file path's extension.
///
/// # Arguments
///
/// * `path` - The file path from which to extract the extension.
///
/// # Returns
///
/// Returns the corresponding `SerdeFormat` if the extension is recognized.
///
/// # Errors
///
/// Returns a `SerializationError` if:
/// - The file has no extension (`missing_extension`).
/// - The extension is not supported (`unsupported_extension`).
///
/// # Examples
///
/// ```rust
/// let format = format_from_path("config.yaml")?;
/// assert_eq!(format, SerdeFormat::Yaml);
/// ```
fn format_from_path(path: impl AsRef<std::path::Path>) -> Result<SerdeFormat, SerializationError> {
    let path = path.as_ref();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| SerializationError::missing_extension())?;

    ext.parse::<SerdeFormat>()
        .map_err(|_| SerializationError::unsupported_extension(ext))
}
