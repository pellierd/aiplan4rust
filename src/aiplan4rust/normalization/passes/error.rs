//! Defines errors that can occur during the normalization pass of the compiler.
//!
//! The `NormalizationPassError` enum encapsulates errors that might be encountered
//! during semantic analysis and transformation phases, including arena allocation issues,
//! syntax tree inconsistencies, and interner-related failures.

use thiserror::Error;

use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents errors that may occur during the normalization pass of the compiler.
///
/// This error type wraps various underlying error types from different stages of the compiler pipeline,
/// including arena allocation, syntax tree analysis, and symbol interning.
///
/// Each variant uses the `#[from]` attribute to allow automatic conversion via the `?` operator.
#[derive(Debug, Error)]
pub enum NormalizationPassError {
    /// An error arising from semantic analysis related to memory arena allocation.
    ///
    /// Typically indicates a failure in allocating or managing memory in the arena during normalization.
    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// An error arising from the syntax tree structure or its semantic validation.
    ///
    /// May occur if the tree is malformed, contains invalid constructs,
    /// or fails invariant checks during normalization.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error related to symbol interning.
    ///
    /// This may happen when a symbol is not found in the interner,
    /// or if interned data is accessed or interpreted incorrectly.
    #[error(transparent)]
    Interner(#[from] InternerError),
}
