//! This module provides functionality for validating the structure and correctness of AST nodes.
//!
//! It consists of two main submodules:
//! - [`error`]: Defines the various validation errors that can occur during AST checks.
//! - [`checks`]: Implements functions to perform structural validation on AST nodes, such as
//!   verifying children count, child kinds, node content, and more.
//!
//! The module also defines type aliases:
//! - [`WellFormedError`]: Alias for `ValidationError`, used to indicate errors during structural validation.
//! - [`WellNormalizedError`]: Alias for `ValidationError`, used to indicate errors during expr checks.
//!
//! # Usage
//!
//! Use the functions in the [`checks`] module to validate AST nodes.
//! On failure, these functions return a [`WellFormedError`] describing the issue.
//!
//! # Examples
//!
//! ```rust
//! use crate::aiplan4rust::validation::{checks, WellFormedError};
//!
//! // Example usage of a check function
//! let result: Result<(), WellFormedError> = checks::check_children_count(children_len, expected_count, &parent_node);
//! if let Err(e) = result {
//!     eprintln!("Validation error: {}", e);
//! }
//! ```
pub mod error;
pub mod checks;

pub type WellFormedError = ValidationError;
pub type WellNormalizedError = ValidationError;

pub use error::ValidationError;
