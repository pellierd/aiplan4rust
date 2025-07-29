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
use crate::aiplan4rust::validation::core::WellNormalizedError;

/// Represents errors that can occur during the normalization process.
///
/// These errors cover various failure scenarios such as structural
/// problems in the AST, allocation failures, or violations of
/// normalization invariants.
///
/// # Variants
/// - `NormalizationPass`: Errors originating from specific normalization passes,
///   typically caused by invalid or unexpected nodes in the syntax tree.
/// - `WellNormalized`: Errors detected during the verification that the AST
///   is well normalized after processing.
#[derive(Debug, Error)]
pub enum NormalizationError {
    /// An error originating from a normalization pass.
    ///
    /// This usually indicates that an invalid or unexpected node
    /// was encountered during the traversal or transformation of
    /// the syntax tree.
    #[error(transparent)]
    NormalizationPass(#[from] NormalizationPassError),

    /// An error indicating that the AST failed the well-normalized check.
    #[error(transparent)]
    WellNormalized(#[from] WellNormalizedError),
}
