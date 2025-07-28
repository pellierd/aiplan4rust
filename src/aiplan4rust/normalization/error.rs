//! Defines the [`NormalizationError`] enum, which represents errors that may occur
//! during the normalization phase of the AI syntax compilation pipeline.
//!
//! The normalization phase is responsible for converting parsed and lifted syntax
//! (such as task networks, methods, and actions) into a more canonical or simplified form
//! suitable for subsequent reasoning, compilation, or execution.
//!
//! This error type encapsulates failures originating from normalization passes, such as:
//!
//! - [`SyntaxTreeError`]: Structural or semantic issues found in the syntax tree.
//! - [`ArenaError`]: Memory allocation or referencing problems within arena-based storage.
//! - [`InternerError`]: Failures related to symbol interning or resolution.
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
//!     Err(NormalizationError::from(
//!         aiplan4rust::normalization::passes::NormalizationPassError::Interner(
//!             some_interner_error
//!         )
//!     ))
//! }
//! ```

use thiserror::Error;

use crate::aiplan4rust::normalization::passes::NormalizationPassError;

/// Represents errors that can occur during normalization.
///
/// This includes structural issues, allocation errors,
/// and internal logic failures.
#[derive(Debug, Error)]
pub enum NormalizationError {

    /// An error originating from the syntax tree layer.
    ///
    /// Typically indicates that an invalid or unexpected node was encountered
    /// while traversing or processing the syntax tree during normalization.
    #[error(transparent)]
    NormalizationPass(#[from] NormalizationPassError),

}
