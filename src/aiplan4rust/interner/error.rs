//! Error types for operations on the StringInterner and related string interning tasks.
//!
//! This module defines the [`InternerError`] enum, representing possible failure
//! modes when interacting with interned strings, such as invalid indices or
//! inconsistent remapping operations performed during linking.
//!
//! These errors capture situations such as:
//! - invalid or stale identifier or literal indices,
//! - missing remapping entries during identifier/literal remapping,
//! - conflicts when multiple items are mapped to the same identifier or literal.
//!
//! They provide useful diagnostics for debugging and error handling across
//! semantic analysis, symbol resolution, and linking.

use thiserror::Error;
use crate::aiplan4rust::lang::{LiteralID, StringID};

/// Represents errors that can occur when working with a [`StringInterner`].
#[derive(Error, Debug)]
pub enum InternerError {

    /// The requested identifier index is out of bounds of the interner's string pool.
    #[error("Invalid identifier index {ident_index}: out of bounds for interner size {interner_size}")]
    InvalidIdent {
        /// The invalid identifier index requested.
        ident_index: usize,
        /// The current size of the interner's string pool.
        interner_size: usize,
    },

    /// The requested literal index is out of bounds of the literal string pool.
    #[error("Invalid literal index {literal_index}: out of bounds for literal pool size {interner_size}")]
    InvalidLiteral {
        /// The invalid literal index requested.
        literal_index: usize,
        /// The current size of the literal string pool.
        interner_size: usize,
    },

    /// A required identifier remapping entry is missing.
    #[error("Missing remap for identifier {0:?}")]
    MissingIdent(StringID),

    /// A required literal remapping entry is missing.
    #[error("Missing remapping entry for literal {0:?}")]
    MissingLiteral(LiteralID),

    /// Two identifiers would be remapped to the same target, causing a conflict.
    #[error("Remapped identifier conflict for {0:?}")]
    Conflict(StringID),
}

impl InternerError {
    /// Creates a new [`InvalidIdent`] error.
    pub fn invalid_ident(ident_index: usize, interner_size: usize) -> Self {
        Self::InvalidIdent { ident_index, interner_size }
    }

    /// Creates a new [`InvalidLiteral`] error.
    pub fn invalid_literal(literal_index: usize, interner_size: usize) -> Self {
        Self::InvalidLiteral { literal_index, interner_size }
    }

    /// Creates a new [`MissingLiteral`] error.
    pub fn missing_literal(literal: LiteralID) -> Self {
        Self::MissingLiteral(literal)
    }

    /// Creates a new [`MissingIdent`] error.
    pub fn missing_ident(id: StringID) -> Self {
        Self::MissingIdent(id)
    }

    /// Creates a new [`Conflict`] error for an identifier.
    pub fn conflict(id: StringID) -> Self {
        Self::Conflict(id)
    }
}
