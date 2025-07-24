use thiserror::Error;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents errors specific to the `lang` module.
///
/// This enum captures the possible error cases that can arise
/// within the language processing components, including errors
/// propagated from the syntax tree system as well as internal errors.
///
/// # Variants
///
/// - [`SyntaxTree`]: Wraps errors originating from the syntax tree subsystem.
/// - [`InternalError`]: Represents generic internal errors with a message.
#[derive(Debug, Error)]
pub enum LangError {
    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// A generic internal error with a descriptive message.
    #[error("Internal Error: {0}")]
    InternalError(String),
}

impl LangError {
    /// Creates a new [`InternalError`] with the given message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::aiplan4rust::lang::LangError;
    /// let err = LangError::internal_error("Something went wrong");
    /// assert_eq!(format!("{}", err), "Internal Error: Something went wrong");
    /// ```
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }
}
