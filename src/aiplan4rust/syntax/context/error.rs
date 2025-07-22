use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;

/// Represents all possible errors that may occur when manipulating the AST
/// or its associated parsing context.
///
/// This error type wraps lower-level issues such as arena allocation errors,
/// as well as internal logic errors that might arise during AST construction
/// or transformation.
#[derive(Error, Debug)]
pub enum ParseContextError {
    /// An error occurred during arena allocation or manipulation.
    ///
    /// This typically indicates a problem allocating or retrieving nodes
    /// from the AST arena structure.
    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    /// A general-purpose internal error that should not occur in normal usage.
    ///
    /// Use this variant to signal logic errors, invariant violations,
    /// or other unexpected states in the parsing context.
    #[error("Internal AST error: {0}")]
    InternalError(String),
}

impl ParseContextError {
    /// Helper to create a new [`ParseContextError::InternalError`] from a message.
    ///
    /// This is intended for signaling unexpected or inconsistent parser states
    /// that should be handled by the developer.
    ///
    /// # Example
    /// ```
    /// let err = ParseContextError::internal_error("Unexpected null node");
    /// ```
    pub fn internal_error(msg: impl Into<String>) -> Self {
        ParseContextError::InternalError(msg.into())
    }
}
