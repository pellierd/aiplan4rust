//! Defines the [`NormalizationError`] enum, which represents errors that may occur
//! during the normalization phase of the AI syntax compilation pipeline.
//!
//! The normalization phase is responsible for converting parsed and lifted syntax
//! (such as objects, constants, and types) into a more canonical or simplified form
//! suitable for subsequent reasoning, compilation, or execution.
//!
//! This error type encapsulates failures originating from normalization finalization, such as:
//!
//! - [`SyntaxTreeError`]: Structural issues found while navigating the tree.
//! - [`NormalizationPassError`]: Failures during specific transformation finalization (e.g., merging duplicates).
//! - [`WellNormalizedError`]: Violations of normalization invariants detected during final validation.
//!
//! All error variants support automatic conversion via `#[from]`, making
//! [`NormalizationError`] convenient to propagate in `Result<T, _>` chains.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::normalization::NormalizationError;
//!
//! fn normalize_something() -> Result<(), NormalizationError> {
//!     // The ? operator automatically converts underlying errors into NormalizationError
//!     perform_pass()?;
//!     Ok(())
//! }
//! ```

use crate::aiplan4rust::compiler::normalization::passes::NormalizationPassError;
use crate::aiplan4rust::compiler::normalization::validation::WellNormalizedError;
use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::compiler::syntax::ast::AstKind;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// Represents errors that can occur during the normalization process.
///
/// These errors cover various failure scenarios such as structural problems
/// in the AST, errors during transformation finalization, or failures during the
/// final "well-normalized" verification.
#[derive(Debug, Error)]
pub enum NormalizationError {
    /// Errors originating from the underlying syntax tree or arena storage.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// An error originating from a specific normalization pass.
    ///
    /// This usually indicates that an invalid or unexpected node was encountered
    /// during the traversal or transformation of the syntax tree (e.g., during
    /// duplicate declaration merging).
    #[error(transparent)]
    NormalizationPass(#[from] NormalizationPassError),

    /// An error indicating that the AST failed the "well-normalized" validation check.
    ///
    /// This check is performed after all normalization finalization to ensure the
    /// resulting AST is in a valid, canonical state.
    #[error(transparent)]
    WellNormalized(#[from] WellNormalizedError),

    /// The AST root node is missing. This usually indicates a corrupted
    /// syntax tree or a failed preceding transformation.
    #[error("Normalization failed: AST has no root node")]
    MissingRoot,

    /// The AST root node has an invalid kind for normalization (expected Domain or Problem).
    #[error("Normalization failed: AST root must be a Domain or a Problem, found {0:?}")]
    IncompatibleRoot(AstKind),
}

impl NormalizationError {
    /// Creates a `MissingRoot` error.
    ///
    /// Should be used when `arena.root_node()` returns `None` unexpectedly.
    pub fn missing_root() -> Self {
        Self::MissingRoot.trace()
    }

    /// Creates an `IncompatibleRoot` error.
    ///
    /// # Arguments
    /// * `found` - The actual [`AstKind`] of the root that was found instead of Domain/Problem.
    pub fn incompatible_root(found: AstKind) -> Self {
        Self::IncompatibleRoot(found).trace()
    }
}

impl Traceable for NormalizationError {}
