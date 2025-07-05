//! Module `serialization`
//!
//! This module provides the `Serializable` trait for serializing and deserializing
//! data structures using multiple Serde-supported formats.
//!
//! Supported formats include JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! The trait handles serialization to/from strings and files, with error handling
//! via `ParserInternalError`.

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::serialization::format::Format;
use base64::{engine::general_purpose, Engine as _};
use serde::{de::DeserializeOwned, Serialize};

/// Trait for serializing and deserializing objects using Serde-supported formats.
///
/// This trait supports formats: JSON, YAML, TOML, CBOR, and MessagePack.
///
/// It provides methods to serialize/deserialize to/from strings and files,
/// returning errors wrapped in `ParserInternalError` on failure.
pub trait Serializable: Serialize + DeserializeOwned {
    /// Serializes the object into a string of the given format.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, `MessagePack`).
    ///
    /// # Returns
    ///
    /// A `String` representing the serialized object, or an error if serialization fails.
    fn serialize_to_string(&self, format: Format) -> Result<String, ParserInternalError> {
        match format {
            Format::Json => serde_json::to_string_pretty(self)
                .map_err(|e| ParserInternalError::new(format!("JSON serialization error: {}", e))),
            Format::Yaml => serde_yaml::to_string(self)
                .map_err(|e| ParserInternalError::new(format!("YAML serialization error: {}", e))),
            Format::Toml => toml::to_string(self)
                .map_err(|e| ParserInternalError::new(format!("TOML serialization error: {}", e))),
            Format::Cbor => {
                let bytes = serde_cbor::to_vec(self).map_err(|e| {
                    ParserInternalError::new(format!("CBOR serialization error: {}", e))
                })?;
                Ok(general_purpose::STANDARD.encode(&bytes))
            }
            Format::MessagePack => {
                let bytes = rmp_serde::to_vec(self).map_err(|e| {
                    ParserInternalError::new(format!("MessagePack serialization error: {}", e))
                })?;
                Ok(general_purpose::STANDARD.encode(&bytes))
            }
        }
    }

    /// Serializes the object and writes it to a file in the specified format.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format.
    /// * `path` - File path where the serialized data will be written.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if writing or serialization fails.
    fn serialize_to_file(&self, format: Format, path: &str) -> Result<(), ParserInternalError> {
        let content = self.serialize_to_string(format)?;
        std::fs::write(path, content)
            .map_err(|e| ParserInternalError::new(format!("File write error: {}", e)))
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
    /// | Format      | Recognized extensions        |
    /// |-------------|-----------------------------|
    /// | JSON        | `.json`                     |
    /// | YAML        | `.yaml`, `.yml`             |
    /// | TOML        | `.toml`                     |
    /// | CBOR        | `.cbor`                     |
    /// | MessagePack | `.msgpack`, `.messagepack`  |
    ///
    /// # Errors
    ///
    /// Returns a `ParserInternalError` if:
    /// - The file extension is missing or unknown.
    /// - Serialization fails.
    /// - Writing to the file fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Serialize to JSON due to `.json` extension
    /// obj.serialize_to_file_auto_format("output.json")?;
    ///
    /// // Serialize to YAML due to `.yaml` extension
    /// obj.serialize_to_file_auto_format("config.yaml")?;
    /// ```
    fn serialize_to_file_auto_format(&self, path: &str) -> Result<(), ParserInternalError> {
        let format = Self::format_from_path(path)?;
        self.serialize_to_file(format, path)
    }

    /// Deserializes an object from a string in the specified format.
    ///
    /// # Arguments
    ///
    /// * `s` - String slice containing the serialized data.
    /// * `format` - Format of the serialized data.
    ///
    /// # Returns
    ///
    /// The deserialized object on success, or an error on failure.
    fn deserialize_from_str(s: &str, format: Format) -> Result<Self, ParserInternalError>
    where
        Self: Sized,
    {
        match format {
            Format::Json => serde_json::from_str(s).map_err(|e| {
                ParserInternalError::new(format!("JSON deserialization error: {}", e))
            }),
            Format::Yaml => serde_yaml::from_str(s).map_err(|e| {
                ParserInternalError::new(format!("YAML deserialization error: {}", e))
            }),
            Format::Toml => toml::from_str(s).map_err(|e| {
                ParserInternalError::new(format!("TOML deserialization error: {}", e))
            }),
            Format::Cbor => {
                let bytes = general_purpose::STANDARD.decode(s).map_err(|e| {
                    ParserInternalError::new(format!("CBOR base64 decode error: {}", e))
                })?;
                serde_cbor::from_slice(&bytes).map_err(|e| {
                    ParserInternalError::new(format!("CBOR deserialization error: {}", e))
                })
            }
            Format::MessagePack => {
                let bytes = general_purpose::STANDARD.decode(s).map_err(|e| {
                    ParserInternalError::new(format!("MessagePack base64 decode error: {}", e))
                })?;
                rmp_serde::from_slice(&bytes).map_err(|e| {
                    ParserInternalError::new(format!("MessagePack deserialization error: {}", e))
                })
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
    /// The deserialized object on success, or an error on failure.
    fn deserialize_from_file(path: &str, format: Format) -> Result<Self, ParserInternalError>
    where
        Self: Sized,
    {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ParserInternalError::new(format!("File read error: {}", e)))?;
        Self::deserialize_from_str(&content, format)
    }

    /// Deserialize an object from a file, automatically detecting the format
    /// from the file extension.
    ///
    /// # Supported formats and their extensions:
    ///
    /// | Format      | Recognized extensions        |
    /// |-------------|-----------------------------|
    /// | JSON        | `.json`                     |
    /// | YAML        | `.yaml`, `.yml`             |
    /// | TOML        | `.toml`                     |
    /// | CBOR        | `.cbor`                     |
    /// | MessagePack | `.msgpack`, `.messagepack`  |
    ///
    /// # Errors
    ///
    /// Returns a `ParserInternalError` if:
    /// - The file extension is missing or unrecognized.
    /// - Reading the file fails.
    /// - Deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj = MyType::deserialize_from_file_auto_format("config.yaml")?;
    /// ```
    fn deserialize_from_file_auto_format(path: &str) -> Result<Self, ParserInternalError>
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
    /// Returns the corresponding `Format` if the extension is recognized,
    /// otherwise returns a `ParserInternalError`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file has no extension.
    /// - The extension does not correspond to a supported format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let format = format_from_path("config.yaml")?;
    /// assert_eq!(format, Format::Yaml);
    /// ```
    fn format_from_path(path: &str) -> Result<Format, ParserInternalError> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| ParserInternalError::new("File has no extension".to_string()))?;

        ext.parse::<Format>()
    }
}
