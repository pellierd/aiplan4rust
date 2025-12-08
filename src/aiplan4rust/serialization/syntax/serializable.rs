//! Module providing the `Serializable` trait for syntax structures requiring an interner.
//!
//! This module defines the `Serializable` trait, which enables serialization and
//! deserialization of AST-like structures that rely on a [`StringInterner`] for symbol management.
//!
//! # Supported Formats
//!
//! The trait supports serialization and deserialization to/from multiple formats, including:
//! - JSON
//! - YAML
//! - TOML
//! - CBOR
//! - MessagePack
//!
//! # Error Handling
//!
//! Format inference from file paths uses [`SerializationError`], which includes:
//! - `MissingExtensionError` for paths without extensions.
//! - `UnsupportedExtensionError` for unrecognized file extensions.
//!
//! # Usage
//!
//! Implementors of `Serializable` must provide implementations for:
//! - Serializing to string or file,
//! - Deserializing from string or file,
//! - Inferring format from file path.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::serialization::Serializable;
//! use crate::aiplan4rust::interner::StringInterner;
//!
//! // Assuming `MyAst` implements Serializable:
//! let interner = StringInterner::new();
//! let ast = MyAst::new();
//! let serialized = ast.serialize_to_string(&interner)?;
//! let deserialized = MyAst::deserialize_from_str(&serialized, &mut interner)?;
//! ```
//!
//! [`StringInterner`]: crate::aiplan4rust::interner::StringInterner
//! [`SerializationError`]: crate::aiplan4rust::serialization::SerializationError

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::serialization::syntax::SyntaxFormat;
use crate::aiplan4rust::syntax::SyntaxDisplay;

/// Trait for serializing and deserializing syntax structures that require an [`Interner`].
///
/// This trait provides functionality to serialize objects into various formats and
/// deserialize objects from strings or files. The supported serialization formats include:
/// - JSON
/// - YAML
/// - TOML
/// - CBOR
/// - MessagePack
///
/// Serialization and deserialization require an [`StringInterner`] to correctly
/// handle symbol resolution during the process.
///
/// # Error Handling
///
/// All methods return an [`SerializationError`] wrapped in a `Result`.
/// - Serialization and deserialization failures return [`SerializationError`].
/// - Format inference failures return [`SerializationError`], specifically:
///   - [`SerializationError::MissingExtensionError`] if the file has no extension.
///   - [`SerializationError::UnsupportedExtensionError`] if the extension is unknown.
///
/// [`Interner`]: crate::aiplan4rust::interner::StringInterner
pub trait Serializable: SyntaxDisplay {
    /// Serializes the object into a string using the provided `interner`.
    ///
    /// # Arguments
    ///
    /// * `interner` - The `StringInterner` instance used for symbol resolution.
    ///
    /// # Returns
    ///
    /// A `String` containing the serialized representation of the object.
    ///
    /// # Errors
    ///
    /// Returns an [`SerializationError`] if serialization fails.
    fn serialize_to_string(
        &self,
        interner: &StringInterner,
    ) -> Result<String, SerializationError>;

    /// Serializes the object and writes the output to a file at the specified path.
    ///
    /// # Arguments
    ///
    /// * `interner` - The `StringInterner` used for symbol resolution.
    /// * `path` - The file system path where the serialized data will be written.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the serialization and write succeed.
    ///
    /// # Errors
    ///
    /// Returns an [`SerializationError`] if serialization or file writing fails.
    fn serialize_to_file(
        &self,
        interner: &StringInterner,
        path: &str,
    ) -> Result<(), SerializationError>;

    /// Deserializes an instance from a string slice.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice containing serialized data.
    /// * `interner` - A mutable reference to a `StringInterner` for resolving symbols.
    ///
    /// # Returns
    ///
    /// The deserialized instance of the implementing type.
    ///
    /// # Errors
    ///
    /// Returns an [`SerializationError`] if deserialization fails.
    fn deserialize_from_str(
        s: &str,
        interner: &mut StringInterner,
    ) -> Result<Self, SerializationError>
    where
        Self: Sized;

    /// Deserializes an instance from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file containing serialized data.
    /// * `interner` - A mutable reference to a `StringInterner` for resolving symbols.
    ///
    /// # Returns
    ///
    /// The deserialized instance of the implementing type.
    ///
    /// # Errors
    ///
    /// Returns an [`SerializationError`] if reading the file or deserialization fails.
    fn deserialize_from_file(
        path: &str,
        interner: &mut StringInterner,
    ) -> Result<Self, SerializationError>
    where
        Self: Sized;

    /// Infers the serialization format from a file path's extension.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path whose extension is used to determine the format.
    ///
    /// # Returns
    ///
    /// A `PlanningFormat` representing the inferred format.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializationError`] if:
    /// - The file has no extension (`MissingExtensionError`).
    /// - The extension is not recognized as a supported format (`UnsupportedExtensionError`).
    fn format_from_path(path: &str) -> Result<SyntaxFormat, SerializationError> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SerializationError::missing_extension())?;

        ext.parse::<SyntaxFormat>()
            .map_err(|_| SerializationError::unsupported_extension(ext))
    }
}
