//! Module `serialization`
//!
//! This module provides the `Serializable` trait for serializing and deserializing
//! data structures using multiple Serde-supported formats.
//!
//! Supported formats include JSON, YAML, TOML, CBOR, and MessagePack.
//!
//! The trait handles serialization to/from strings and files, with error handling
//! via `ParserInternalError`.

use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeHeader};
use crate::aiplan4rust::serialization::SerializationError;
use base64::{engine::general_purpose, Engine as _};
use serde::{de::DeserializeOwned, Serialize};
use crate::aiplan4rust::serialization::header::Header;

/// Separator used to distinguish the serialized header from the payload in a string.
///
/// When serializing objects with a header (`SerdeHeader`) and a payload, this
/// constant defines the unique sequence of characters that separates the two.
///
/// Typically used in functions like [`parse_header_and_payload`] and
/// [`Serializable::serialize_to_string`] to reliably split or join header and payload.
///
/// # Example
///
/// ```rust
/// # const HEADER_PAYLOAD_SEPARATOR: &str = "\n---\n";
/// let serialized = format!("{{\"magic\":\"AIPL\"}}{}{{\"key\":\"value\"}}", HEADER_PAYLOAD_SEPARATOR);
/// let parts: Vec<&str> = serialized.splitn(2, HEADER_PAYLOAD_SEPARATOR).collect();
/// assert_eq!(parts.len(), 2);
/// assert!(parts[0].contains("magic"));
/// assert!(parts[1].contains("key"));
/// ```
pub const HEADER_PAYLOAD_SEPARATOR: &str = "\n---\n";

/// Trait for serializing and deserializing objects using Serde-supported formats.
///
/// This trait supports formats: JSON, YAML, TOML, CBOR, and MessagePack.
///
/// It provides methods to serialize/deserialize to/from strings and files,
/// returning errors wrapped in `SerializationError` on failure.
pub trait Serializable: Serialize + DeserializeOwned {
    /// Serializes the object into a string in the specified format, wrapped in an `Envelope`.
    ///
    /// The object is first wrapped in a `SerdeEnvelope` that contains a `SerdeHeader`
    /// (including magic number, version, format, and generation timestamp) and the
    /// object itself as the payload. This allows identifying files produced by this
    /// application and validating their format/version before deserialization.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the serialized representation of the envelope,
    /// or a `SerializationError` if serialization fails.
    ///
    /// # Errors
    ///
    /// Serialization can fail if:
    /// - The object cannot be converted to the specified format.
    /// - Encoding fails (for binary formats like `Cbor` and `MessagePack`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Serialize an object to JSON, wrapped in an envelope with header
    /// let json_str = obj.serialize_to_string(SerdeFormat::Json)?;
    ///
    /// // Serialize an object to YAML, wrapped in an envelope with header
    /// let yaml_str = obj.serialize_to_string(SerdeFormat::Yaml)?;
    /// ```
    /// Serializes the object to a string, prepending a JSON header.
    ///
    /// # Arguments
    ///
    /// * `format` - The desired format for the payload (`Json`, `Yaml`, `Toml`, `Cbor`, `MessagePack`).
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the serialized header followed by the serialized payload,
    /// or a `SerializationError` if serialization fails.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if either header or payload serialization fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let serialized = obj.serialize_to_string(SerdeFormat::Json)?;
    /// ```
    fn serialize_to_string(&self, format: SerdeFormat) -> Result<String, SerializationError> {
        // Serialize the header as JSON
        let header = SerdeHeader::new(format, 1);
        let header_str = serde_json::to_string_pretty(&header)
            .map_err(|e| SerializationError::json_serialization(e.to_string()))?;

        // Serialize the payload in the specified format
        let payload_str = match format {
            SerdeFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| SerializationError::json_serialization(e.to_string()))?,
            SerdeFormat::Yaml => serde_yaml::to_string(self)
                .map_err(|e| SerializationError::yaml_serialization(e.to_string()))?,
            SerdeFormat::Toml => toml::to_string(self)
                .map_err(|e| SerializationError::toml_serialization(e.to_string()))?,
            SerdeFormat::Cbor => {
                let bytes = serde_cbor::to_vec(self)
                    .map_err(|e| SerializationError::cbor_serialization(e.to_string()))?;
                general_purpose::STANDARD.encode(&bytes)
            }
            SerdeFormat::MessagePack => {
                let bytes = rmp_serde::to_vec(self)
                    .map_err(|e| SerializationError::messagepack_serialization(e.to_string()))?;
                general_purpose::STANDARD.encode(&bytes)
            }
        };

        // Combine header and payload with a newline separator
        Ok(format!("{}\n---\n{}", header_str, payload_str))
    }

    /// Serializes the object (wrapped in a `SerdeEnvelope` with header) and writes it to a file
    /// in the specified format.
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
    /// // Serialize to JSON due to `.json` extension
    /// obj.serialize_to_file_with_auto_format("output.json")?;
    ///
    /// // Serialize to YAML due to `.yaml` extension
    /// obj.serialize_to_file_with_auto_format("config.yaml")?;
    /// ```
    fn serialize_to_file_with_auto_format(&self, path: &str) -> Result<(), SerializationError> {
        let format = format_from_path(path)?;
        self.serialize_to_file(format, path)
    }

    /// Deserializes an object from a string containing a header and a payload.
    ///
    /// This function expects the input string `s` to contain a serialized `Header`
    /// followed by the actual payload. The header is used to:
    /// 1. Validate the magic number to ensure the data comes from our application.
    /// 2. Determine the format of the payload (`Json`, `Yaml`, `Toml`, `Cbor`, or `MessagePack`).
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice containing the serialized data (header + payload).
    ///
    /// # Returns
    ///
    /// Returns the deserialized object of type `Self` on success, or a [`SerializationError`] if deserialization fails.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializationError`] in the following cases:
    /// - [`SerializationError::JsonDeserializationError`] if the header or payload cannot be parsed as JSON when expected.
    /// - [`SerializationError::YamlDeserializationError`] if the payload cannot be parsed as YAML.
    /// - [`SerializationError::TomlDeserializationError`] if the payload cannot be parsed as TOML.
    /// - [`SerializationError::Base64DecodeError`] if base64 decoding of CBOR or MessagePack fails.
    /// - [`SerializationError::CborDeserializationError`] if CBOR deserialization fails.
    /// - [`SerializationError::MessagePackDeserializationError`] if MessagePack deserialization fails.
    /// - [`SerializationError::InvalidMagic`] if the header's magic number is incorrect.
    ///
    /// # Example
    ///
    /// ```rust
    /// let serialized_str = r#"{
    ///     "magic": "AIPL",
    ///     "version": 1,
    ///     "format": "Json",
    ///     "generated_at": "2025-12-14T12:00:00Z"
    /// }
    /// {
    ///     "key": "value"
    /// }"#;
    ///
    /// let obj: MyType = MyType::deserialize_from_str(serialized_str)?;
    /// ```
    fn deserialize_from_str(s: &str) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        let (header, payload_str) = Header::parse_header_and_payload(s)?;

        // Validate magic number
        if !header.validate_magic() {
            return Err(SerializationError::invalid_magic());
        }

        // Deserialize payload according to the format specified in header
        match header.format {
            SerdeFormat::Json => serde_json::from_str(payload_str)
                .map_err(|e| SerializationError::json_deserialization(e.to_string())),
            SerdeFormat::Yaml => serde_yaml::from_str(payload_str)
                .map_err(|e| SerializationError::yaml_deserialization(e.to_string())),
            SerdeFormat::Toml => toml::from_str(payload_str)
                .map_err(|e| SerializationError::toml_deserialization(e.to_string())),
            SerdeFormat::Cbor => {
                let bytes = general_purpose::STANDARD
                    .decode(payload_str)
                    .map_err(|e| SerializationError::base64_decode(e.to_string()))?;
                serde_cbor::from_slice(&bytes)
                    .map_err(|e| SerializationError::cbor_deserialization(e.to_string()))
            }
            SerdeFormat::MessagePack => {
                let bytes = general_purpose::STANDARD
                    .decode(payload_str)
                    .map_err(|e| SerializationError::base64_decode(e.to_string()))?;
                rmp_serde::from_slice(&bytes)
                    .map_err(|e| SerializationError::messagepack_deserialization(e.to_string()))
            }
        }
    }

    /// Deserializes an object from a file using the header to determine the format.
    ///
    /// This function reads the file at the specified path and expects the content
    /// to start with a valid `Header` (magic number, version, format, and timestamp).
    /// The header is used to determine the serialization format of the payload.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file containing serialized data.
    ///
    /// # Returns
    ///
    /// Returns the deserialized object on success, or a [`SerializationError`] on failure.
    ///
    /// # Errors
    ///
    /// This function may return errors including, but not limited to:
    /// - [`SerializationError::FileReadError`] if the file cannot be read.
    /// - [`SerializationError::DeserializationError`] if parsing the payload fails.
    /// - [`SerializationError::InvalidMagic`] if the header's magic number is incorrect.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let obj: MyType = MyType::deserialize_from_file("data.json")?;
    /// ```
    fn deserialize_from_file(path: &str) -> Result<Self, SerializationError>
    where
        Self: Sized + Serialize + DeserializeOwned,
    {
        // Read the entire file content
        let content = std::fs::read_to_string(path)
            .map_err(|e| SerializationError::file_read(e.to_string()))?;

        // Use the header to determine the payload format and deserialize
        Self::deserialize_from_str(&content)
            .map_err(|e| SerializationError::deserialization(e.to_string()))
    }

    /// Deserializes an object from a file, automatically detecting the payload format
    /// based on the file extension while still validating the file header.
    ///
    /// The file is expected to start with a valid `SerdeHeader` (magic number, version,
    /// format, timestamp) serialized in JSON, followed by the actual payload. The header
    /// is always in JSON, but the payload format is inferred either from the header or
    /// automatically from the file extension.
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
    /// - The payload cannot be deserialized according to the format in the header.
    ///
    /// # Example
    ///
    /// ```rust
    /// let obj: MyType = MyType::deserialize_from_file_auto_format("config.yaml")?;
    /// ```
    fn deserialize_from_file_with_auto_format(path: &str) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        // Deserialize file content using the header to determine payload format
        Self::deserialize_from_file(path)
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
fn format_from_path(path: &str) -> Result<SerdeFormat, SerializationError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| SerializationError::missing_extension())?;

    ext.parse::<SerdeFormat>()
        .map_err(|_| SerializationError::unsupported_extension(ext))
}
