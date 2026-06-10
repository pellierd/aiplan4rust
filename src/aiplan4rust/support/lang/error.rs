//! Module `lang_error`
//!
//! This module defines the [`LangError`] enum, representing errors specific to the `lang` module.
//!
//! It encapsulates errors arising from language processing components,
//! including those propagated from the syntax tree system, as well as internal errors.
//!
//! # Error Variants
//!
//! - [`SyntaxTree`]: Wraps errors originating from the syntax tree subsystem.
//! - [`InternalError`]: Represents generic internal errors with a descriptive message.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lang::LangError;
//! use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
//!
//! fn example() -> Result<(), LangError> {
//!     let syntax_error = SyntaxTreeError::some_variant();
//!     Err(LangError::from(syntax_error))
//! }
//! ```
use crate::aiplan4rust::support::lang::ObjectId;
use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

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

    #[error("Unexpected Object: expected an ObjectFluent but found ObjectID {0}")]
    UnexpectedObject(ObjectId),
}
