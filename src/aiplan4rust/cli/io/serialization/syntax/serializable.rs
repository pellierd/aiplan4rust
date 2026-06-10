//! Module providing the `Serializable` trait for syntax structures requiring an interner.
//!
//! This module defines the [`Serializable`] trait, which enables serialization of AST-like
//! or syntax structures that rely on a [`StringInterner`] for symbol management.
//!
//! # Supported Use Case
//!
//! The trait is intended for objects such as normalized domains or problems, where you want
//! to serialize them to a string or a file for reproducibility, caching, or verification
//! by re-parsing the serialized content.
//!
//! Currently, deserialization is **not** required, but could be added later if needed.
//!
//! # Error Handling
//!
//! All methods return a [`SerializationError`] wrapped in a `Result`.
//! File-related errors (read/write) and unsupported formats/extensions are handled explicitly.
//!
//! [`StringInterner`]: crate::aiplan4rust::support::interner::SymbolInterner
//! [`SerializationError`]: crate::aiplan4rust::cli::io::serialization::SerializationError
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::serialization::Serializable;
//! use crate::aiplan4rust::interner::StringInterner;
//! use crate::aiplan4rust::lir::problem::DomainDef;
//!
//! # let domain_def: DomainDef = todo!();
//! let interner = domain_def.interner();
//!
//! // Serialize to string
//! let serialized = domain_def.serialize_to_string(interner)?;
//!
//! // Serialize to file
//! domain_def.serialize_to_file(interner, "domain.txt")?;
//! ```

use crate::aiplan4rust::cli::io::serialization::SerializationError;
use crate::aiplan4rust::compiler::syntax::SyntaxDisplay;
use std::fs;
use std::io::Write;

/// Trait for serializing syntax structures that contains a [`StringInterner`], i.e.,
/// [`DomainDef`], [`ProblemDef`] or [`Ast`]?
///
/// This trait provides functionality to convert objects to a string representation
/// or write them directly to a file. It is intended for structures like normalized
/// ASTs, domain definitions, or problems in PDDL-like syntax.
///
pub trait Serializable: SyntaxDisplay {
    /// Serializes the object into a string using the provided interner.
    ///
    /// # Returns
    ///
    /// A `String` containing the serialized representation.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializationError`] if serialization fails.
    fn serialize_to_string(&self) -> Result<String, SerializationError>;

    /// Serializes the object and writes the output to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file to write the serialized content.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the file was written successfully.
    ///
    /// # Errors
    ///
    /// Returns a [`SerializationError`] if serialization or file writing fails.
    fn serialize_to_file(&self, path: &str) -> Result<(), SerializationError> {
        // Serialize to string first
        let content = self.serialize_to_string()?;

        // Attempt to create/open the file
        let mut file = fs::File::create(path)
            .map_err(|e| SerializationError::file_write(format!("{}: {}", path, e)))?;

        // Write the serialized content
        file.write_all(content.as_bytes())
            .map_err(|e| SerializationError::file_write(format!("{}: {}", path, e)))?;

        Ok(())
    }
}
