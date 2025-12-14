//! Module `serialization`
//!
//! This module provides the `Serializable` trait for serializing and deserializing
//! data structures using multiple Serde-supported formats.
//!
//! Supported formats include JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! The trait handles serialization to/from strings and files, with error handling
//! via `ParserInternalError`.

use crate::aiplan4rust::serialization::serde::SerdeFormat;
use base64::{engine::general_purpose, Engine as _};
use serde::{de::DeserializeOwned, Serialize};
use crate::aiplan4rust::serialization::SerializationError;

/// Trait for serializing and deserializing objects using Serde-supported formats.
///
/// This trait supports formats: JSON, YAML, TOML, CBOR, and MessagePack.
///
/// It provides methods to serialize/deserialize to/from strings and files,
/// returning errors wrapped in `SerializationError` on failure.
pub trait Serializable: Serialize + DeserializeOwned {
    /// Serializes the object into a string in the specified format.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the serialized representation of the object,
    /// or a `SerializationError` if serialization fails.
    ///
    /// # Errors
    ///
    /// Serialization can fail if the object cannot be converted to the specified format,
    /// or if encoding fails (for binary formats like `Cbor` and `MessagePack`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// let json_str = obj.serialize_to_string(SerdeFormat::Json)?;
    /// let yaml_str = obj.serialize_to_string(SerdeFormat::Yaml)?;
    /// ```
    fn serialize_to_string(&self, format: SerdeFormat) -> Result<String, SerializationError> {
        match format {
            SerdeFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| SerializationError::JsonSerializationError(e.to_string())),

            SerdeFormat::Yaml => serde_yaml::to_string(self)
                .map_err(|e| SerializationError::YamlSerializationError(e.to_string())),

            SerdeFormat::Toml => toml::to_string(self)
                .map_err(|e| SerializationError::TomlSerializationError(e.to_string())),

            SerdeFormat::Cbor => {
                let bytes = serde_cbor::to_vec(self)
                    .map_err(|e| SerializationError::CborSerializationError(e.to_string()))?;
                Ok(general_purpose::STANDARD.encode(&bytes))
            }

            SerdeFormat::MessagePack => {
                let bytes = rmp_serde::to_vec(self)
                    .map_err(|e| SerializationError::MessagePackSerializationError(e.to_string()))?;
                Ok(general_purpose::STANDARD.encode(&bytes))
            }
        }
    }

    /// Serializes the object and writes it to a file in the specified format.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    /// * `path` - The file path where the serialized data will be written.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the object is successfully serialized and written to the file,
    /// or a `SerializationError` if serialization or file writing fails.
    ///
    /// # Errors
    ///
    /// This function can return:
    /// - `SerializationError::JsonSerializationError`, `YamlSerializationError`, etc., if serialization fails.
    /// - `SerializationError::FileWriteError` if writing to the specified file fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// obj.serialize_to_file(SerdeFormat::Json, "output.json")?;
    /// ```
    fn serialize_to_file(&self, format: SerdeFormat, path: &str) -> Result<(), SerializationError> {
        let content = self.serialize_to_string(format)?;
        std::fs::write(path, content).map_err(|e| SerializationError::file_write(e.to_string()))?;
        Ok(())
    }

    /// Serializes the object and writes it to a file, automatically inferring
    /// the serialization format from the file extension.
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
    /// // Serialize to JSON due to `.json` extension
    /// obj.serialize_to_file_auto_format("output.json")?;
    ///
    /// // Serialize to YAML due to `.yaml` extension
    /// obj.serialize_to_file_auto_format("config.yaml")?;
    /// ```
    fn serialize_to_file_auto_format(&self, path: &str) -> Result<(), SerializationError> {
        let format = Self::format_from_path(path)?;
        self.serialize_to_file(format, path)
    }

    /// Deserializes an object from a string in the specified serialization format.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice containing the serialized data.
    /// * `format` - The serialization format of the input data.
    ///
    /// # Returns
    ///
    /// Returns the deserialized object on success, or a [`SerializationError`] on failure.
    ///
    /// # Errors
    ///
    /// This function may return errors including, but not limited to:
    /// - [`SerializationError::JsonDeserializationError`] if JSON parsing fails.
    /// - [`SerializationError::YamlDeserializationError`] if YAML parsing fails.
    /// - [`SerializationError::TomlDeserializationError`] if TOML parsing fails.
    /// - [`SerializationError::Base64DecodeError`] if base64 decoding of CBOR or MessagePack data fails.
    /// - [`SerializationError::CborDeserializationError`] if CBOR deserialization fails.
    /// - [`SerializationError::MessagePackDeserializationError`] if MessagePack deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let json_str = r#"{"key":"value"}"#;
    /// let obj = MyType::deserialize_from_str(json_str, SerdeFormat::Json)?;
    /// ```
    fn deserialize_from_str(s: &str, format: SerdeFormat) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        match format {
            SerdeFormat::Json => serde_json::from_str(s)
                .map_err(|e| SerializationError::JsonDeserializationError(e.to_string())),

            SerdeFormat::Yaml => serde_yaml::from_str(s)
                .map_err(|e| SerializationError::YamlDeserializationError(e.to_string())),

            SerdeFormat::Toml => toml::from_str(s)
                .map_err(|e| SerializationError::TomlDeserializationError(e.to_string())),

            SerdeFormat::Cbor => {
                let bytes = general_purpose::STANDARD
                    .decode(s)
                    .map_err(|e| SerializationError::Base64DecodeError(e.to_string()))?;
                serde_cbor::from_slice(&bytes)
                    .map_err(|e| SerializationError::CborDeserializationError(e.to_string()))
            }

            SerdeFormat::MessagePack => {
                let bytes = general_purpose::STANDARD
                    .decode(s)
                    .map_err(|e| SerializationError::Base64DecodeError(e.to_string()))?;
                rmp_serde::from_slice(&bytes)
                    .map_err(|e| SerializationError::MessagePackDeserializationError(e.to_string()))
            }
        }
    }

    /// Deserializes an object from a file in the specified format.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file containing serialized data.
    /// * `format` - Format of the serialized data.
    ///
    /// # Returns
    ///
    /// The deserialized object on success, or a [`SerializationError`] on failure.
    fn deserialize_from_file(path: &str, format: SerdeFormat) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SerializationError::file_read(e.to_string()))?;

        Self::deserialize_from_str(&content, format)
            .map_err(|e| SerializationError::deserialization(e.to_string()))
    }

    /// Deserialize an object from a file, automatically detecting the format
    /// based on the file extension.
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
    /// - Reading the file fails.
    /// - Deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = MyType::deserialize_from_file_auto_format("config.yaml")?;
    /// ```
    fn deserialize_from_file_auto_format(path: &str) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        // Extract extension from file path
        let format = Self::format_from_path(path)?;
        // Deserialize file content according to detected format
        Self::deserialize_from_file(path, format)
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
    fn format_from_path(path: &str) -> Result<SerdeFormat, SerializationError> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SerializationError::missing_extension())?;

        ext.parse::<SerdeFormat>()
            .map_err(|_| SerializationError::unsupported_extension(ext))
    }
}
