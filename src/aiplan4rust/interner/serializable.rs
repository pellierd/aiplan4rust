//! This module defines the `SerializableWithInterner` trait, which extends serialization
//! and deserialization capabilities by incorporating an interner or context during the process.
//!
//! This trait is designed for types that require a serialization context, such as an
//! interner, to correctly (de)serialize data. The generic parameter `I` represents the
//! interner or contextual data needed for these operations.
//!
//! # Overview
//!
//! - `serialize_to_string_with_interner`: Serialize an object to a string (JSON or YAML) using the provided interner.
//! - `serialize_to_file_with_interner`: Serialize an object and write it to a file using the interner.
//! - `deserialize_from_str_with_interner`: Deserialize an object from a string using the interner.
//! - `deserialize_from_file_with_interner`: Deserialize an object from a file using the interner.
//!
//! # Error Handling
//!
//! The trait methods produce detailed errors depending on the format (JSON/YAML) that caused
//! the failure, as well as errors related to file I/O operations.
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use crate::serialization::Format;
//! use crate::interner::SerializableWithInterner;
//! use crate::frontend::ParserInternalError;
//!
//! struct MyInterner { /* ... */ }
//! struct MyType { /* ... */ }
//!
//! impl SerializableWithInterner<MyInterner> for MyType {
//!     fn serialize_to_string_with_interner(&self, format: Format, interner: &MyInterner) -> Result<String, ParserInternalError> {
//!         // implement serialization logic using `interner`
//!         # Ok(String::new())
//!     }
//!
//!     fn deserialize_from_str_with_interner(s: &str, format: Format, interner: &MyInterner) -> Result<Self, ParserInternalError> {
//!         // implement deserialization logic using `interner`
//!         # Ok(MyType { /* ... */ })
//!     }
//! }
//! ```

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::serialization::Format;
use serde::Serialize;

/// Trait extending serialization and deserialization by requiring an interner/context `I`.
///
/// Types implementing this trait can serialize and deserialize themselves with
/// additional contextual information provided by an interner, which is often
/// required for advanced data structures relying on deduplication or symbol tables.
///
/// # Type parameter
/// - `I`: The interner or context type that provides additional data or behavior needed
///        during serialization and deserialization.
///
/// # Errors
/// Methods return `ParserInternalError` with detailed messages depending on:
/// - Serialization or deserialization errors specific to JSON or YAML formats.
/// - File input/output errors during reading or writing.
///
/// # Example
/// See the module-level documentation for usage.
pub trait SerializableWithInterner: Serialize + Sized {
    /// Serializes the object into a string using the specified format and interner.
    ///
    /// # Parameters
    /// - `format`: The output format (e.g., JSON or YAML).
    /// - `interner`: The interner/context used during serialization.
    ///
    /// # Returns
    /// A `Result` containing the serialized string or a `ParserInternalError` on failure.
    fn serialize_to_string_with_interner(
        &self,
        format: Format,
        interner: &StringInterner,
    ) -> Result<String, ParserInternalError>;

    /// Serializes the object and writes it to a file using the specified format and interner.
    ///
    /// The error messages differentiate between serialization errors per format and
    /// file writing errors.
    ///
    /// # Parameters
    /// - `format`: The output format (e.g., JSON or YAML).
    /// - `path`: The file path where to write the serialized data.
    /// - `interner`: The interner/context used during serialization.
    ///
    /// # Returns
    /// A `Result` indicating success or containing a `ParserInternalError` on failure.
    fn serialize_to_file_with_interner(
        &self,
        format: Format,
        path: &str,
        interner: &StringInterner,
    ) -> Result<(), ParserInternalError> {
        let content = self
            .serialize_to_string_with_interner(format, interner)
            .map_err(|e| {
                ParserInternalError::new(format!("Serialization error ({:?}): {}", format, e))
            })?;
        std::fs::write(path, &content)
            .map_err(|e| ParserInternalError::new(format!("File write error: {}", e)))
    }

    /// Deserializes an object from a string using the specified format and interner.
    ///
    /// # Parameters
    /// - `s`: The input string containing serialized data.
    /// - `format`: The input format (e.g., JSON or YAML).
    /// - `interner`: The interner/context used during deserialization.
    ///
    /// # Returns
    /// A `Result` containing the deserialized object or a `ParserInternalError` on failure.
    fn deserialize_from_str_with_interner(
        s: &str,
        format: Format,
        interner: &StringInterner,
    ) -> Result<Self, ParserInternalError>;

    /// Deserializes an object from a file using the specified format and interner.
    ///
    /// The error messages differentiate between file reading errors and
    /// deserialization errors depending on the format.
    ///
    /// # Parameters
    /// - `path`: The file path to read serialized data from.
    /// - `format`: The input format (e.g., JSON or YAML).
    /// - `interner`: The interner/context used during deserialization.
    ///
    /// # Returns
    /// A `Result` containing the deserialized object or a `ParserInternalError` on failure.
    fn deserialize_from_file_with_interner(
        path: &str,
        format: Format,
        interner: &StringInterner,
    ) -> Result<Self, ParserInternalError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ParserInternalError::new(format!("File read error: {}", e)))?;
        Self::deserialize_from_str_with_interner(&content, format, interner).map_err(|e| {
            ParserInternalError::new(format!("Deserialization error ({:?}): {}", format, e))
        })
    }
}
