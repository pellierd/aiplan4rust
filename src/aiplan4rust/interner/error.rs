//! Error types for operations on the StringInterner and related string interning tasks.
//!
//! This module defines the [`InternerError`] enum, representing possible failure
//! modes when interacting with interned strings, such as invalid indices or
//! inconsistent remapping operations performed during linking.
//!
//! These errors capture situations such as:
//! - invalid or stale interner indices,
//! - missing remapping entries during identifier/literal remapping,
//! - conflicts when multiple items are mapped to the same identifier or literal.
//!
//! They provide useful diagnostics for debugging and error handling across
//! semantic analysis, symbol resolution, and linking.

use thiserror::Error;
use crate::aiplan4rust::interner::{Ident, Literal};

/// Represents errors that can occur when working with a [`StringInterner`].
#[derive(Error, Debug)]
pub enum InternerError {
    /// The requested identifier index is out of bounds of the interner's string pool.
    ///
    /// This usually happens when trying to resolve an invalid or stale identifier.
    #[error("Invalid identifier index {ident_index}: out of bounds for interner size {interner_size}")]
    InvalidIdent {
        /// The invalid identifier index requested.
        ident_index: usize,
        /// The current size of the interner's string pool.
        interner_size: usize,
    },

    /// The requested literal index is out of bounds of the literal string pool.
    ///
    /// This usually happens when trying to resolve an invalid or stale literal identifier.
    #[error("Invalid literal index {literal_index}: out of bounds for literal pool size {interner_size}")]
    InvalidLiteral {
        /// The invalid literal index requested.
        literal_index: usize,
        /// The current size of the literal string pool.
        interner_size: usize,
    },

    /// A required identifier remapping entry is missing.
    ///
    /// This occurs when an identifier is expected to be remapped (e.g., during
    /// linking or identifier normalization) but no corresponding entry is found
    /// in the remapping table.
    #[error("Missing remapping entry for identifier {0:?}")]
    MissingRemapIdent(Ident),

    /// A required literal remapping entry is missing.
    ///
    /// This occurs when a literal is expected to be remapped (e.g., during
    /// problem merging or normalization) but no corresponding entry exists.
    #[error("Missing remapping entry for literal {0:?}")]
    MissingRemapLiteral(Literal),

    /// Multiple identifiers or literals were remapped to the same new identifier,
    /// causing a collision—usually a logic error in the linker's remapping tables.
    #[error("Identifier remapping conflict: multiple entries mapped to '{0:?}'")]
    RemappedIdentifierConflict(Ident),
}

impl InternerError {
    /// Creates a new [`InvalidIdent`] error.
    ///
    /// # Arguments
    ///
    /// * `ident_index` - The invalid identifier index that was requested.
    /// * `interner_size` - The current size of the interner's string pool.
    ///
    /// # Returns
    ///
    /// A new `InternerError::InvalidIdent` instance.
    pub fn invalid_ident(ident_index: usize, interner_size: usize) -> Self {
        Self::InvalidIdent { ident_index, interner_size }
    }

    /// Creates a new [`InvalidLiteral`] error.
    ///
    /// # Arguments
    ///
    /// * `literal_index` - The invalid literal index that was requested.
    /// * `interner_size` - The current size of the literal string pool.
    ///
    /// # Returns
    ///
    /// A new `InternerError::InvalidLiteral` instance.
    pub fn invalid_literal(literal_index: usize, interner_size: usize) -> Self {
        Self::InvalidLiteral { literal_index, interner_size }
    }

    /// Creates a new [`MissingRemapIdent`] error.
    ///
    /// # Arguments
    ///
    /// * `ident` - The identifier for which no remapping entry exists.
    ///
    /// # Returns
    ///
    /// A new `InternerError::MissingRemapIdent` instance.
    pub fn missing_remap_ident(ident: Ident) -> Self {
        Self::MissingRemapIdent(ident)
    }

    /// Creates a new [`MissingRemapLiteral`] error.
    ///
    /// # Arguments
    ///
    /// * `literal` - The literal for which no remapping entry exists.
    ///
    /// # Returns
    ///
    /// A new `InternerError::MissingRemapLiteral` instance.
    pub fn missing_remap_literal(literal: Literal) -> Self {
        Self::MissingRemapLiteral(literal)
    }

    /// Creates a new [`RemappedIdentifierConflict`] error.
    ///
    /// # Arguments
    ///
    /// * `ident` - The identifier to which multiple entries were remapped, causing a conflict.
    ///
    /// # Returns
    ///
    /// A new `InternerError::RemappedIdentifierConflict` instance.
    pub fn remapped_identifier_conflict(ident: Ident) -> Self {
        Self::RemappedIdentifierConflict(ident)
    }
}
