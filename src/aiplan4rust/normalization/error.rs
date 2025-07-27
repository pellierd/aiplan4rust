//! Defines the [`NormalizationError`] enum, which represents errors that may occur
//! during the normalization phase of the AI planning compilation pipeline.
//!
//! The normalization phase is responsible for converting parsed and lifted syntax
//! (such as task networks, methods, and actions) into a more canonical or simplified form
//! suitable for subsequent reasoning, compilation, or execution.
//!
//! This error type_checker encapsulates several categories of failure:
//! - [`SyntaxTreeError`]: Structural or semantic issues found in the syntax tree.
//! - [`ArenaError`]: Memory allocation or referencing problems within the arena-based storage.
//! - [`InternerError`]: Failures related to symbol interning or resolution.
//! - Internal errors with descriptive messages, used when no specific variant fits.
//!
//! All error variants support automatic conversion from their underlying types,
//! making [`NormalizationError`] convenient to propagate in `Result<T, _>` chains.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::normalization::NormalizationError;
//!
//! fn normalize_something() -> Result<(), NormalizationError> {
//!     // ...
//!     Err(NormalizationError::internal_error("unexpected case"))
//! }
//! ```

use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents errors that can occur during normalization.
///
/// This includes structural issues, allocation errors,
/// and internal logic failures.
#[derive(Debug, Error)]
pub enum NormalizationError {
    /// A catch-all for unexpected internal errors during normalization.
    ///
    /// This variant is typically used when a more specific error kind is not available.
    #[error("Internal error: {0}")]
    InternalError(String),

    /// An error originating from the syntax tree layer.
    ///
    /// Typically indicates that an invalid or unexpected node was encountered
    /// while traversing or processing the syntax tree during normalization.
    #[error(transparent)]
    Syntax(#[from] SyntaxTreeError),

    /// An error from the arena storage system.
    ///
    /// Arena errors may occur when accessing nodes or allocating space
    /// in the underlying memory structure used to represent syntax elements.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error related to symbol interning.
    ///
    /// This may happen when a symbol is not found in the interner,
    /// or if interned data is used incorrectly.
    #[error(transparent)]
    Interner(#[from] InternerError),
}

impl NormalizationError {
    /// Constructs an [`InternalError`] variant with a custom message.
    ///
    /// Use this when no specific error variant applies but you want to report
    /// a meaningful failure during normalization.
    ///
    /// # Arguments
    /// * `msg` - A string describing the internal error.
    ///
    /// # Returns
    /// A `NormalizationError::InternalError` instance.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        NormalizationError::InternalError(msg.into())
    }
}
