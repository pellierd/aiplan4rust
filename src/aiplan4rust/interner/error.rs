//! Error types for operations on the StringInterner and related string interning tasks.
//!
//! This module defines the [`InternerError`] enum, representing possible failure
//! modes when interacting with interned strings, such as invalid indices or
//! internal consistency violations.
//!
//! # Usage
//!
//! When resolving interned string identifiers, these errors indicate problems
//! such as invalid IDs or unexpected internal states.
//!
//! Each variant provides context useful for debugging and error handling.
//!
//! # Example
//!
//! ```rust
//! use crate::error::InternerError;
//!
//! fn example(id: usize, pool_size: usize) -> Result<(), InternerError> {
//!     if id >= pool_size {
//!         Err(InternerError::invalid_ident { ident_index: id, interner_size: pool_size })
//!     } else {
//!         Ok(())
//!     }
//! }
//! ```

use thiserror::Error;

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

    /// A generic internal error indicating an unexpected or inconsistent state.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl InternerError {
    /// Creates a new [`InvalidIdent`] error.
    pub fn invalid_ident(ident_index: usize, interner_size: usize) -> Self {
        Self::InvalidIdent { ident_index, interner_size }
    }

    /// Creates a new [`InternalError`] with the given message.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }
}
