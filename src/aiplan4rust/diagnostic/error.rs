//! Module defining errors related to diagnostics and rendering.
//!
//! This module provides the [`DiagnosticError`] enum, which represents
//! errors that can occur during diagnostic processing and rendering
//! in the system. It includes both errors originating from I/O
//! operations and potential rendering-specific failures.
//!
//! # Usage
//!
//! When generating or displaying diagnostics, functions can return
//! a `Result<_, DiagnosticError>`. I/O errors are automatically
//! wrapped as `DiagnosticError::Io` via the `From` trait.
//!
//! ```rust
//! use crate::diagnostic::DiagnosticError;
//!
//! fn render_example() -> Result<(), DiagnosticError> {
//!     let mut buffer = Vec::new();
//!     std::io::Write::write_all(&mut buffer, b"Hello")?; // propagated as DiagnosticError::Io
//!     Ok(())
//! }
//! ```
//!
//! This design allows transparent propagation of errors from underlying
//! operations without losing the original context or error details.

use thiserror::Error;

/// Errors related to diagnostic processing and rendering.
#[derive(Error, Debug)]
pub enum DiagnosticError {
    /// Wraps any I/O errors encountered during diagnostic handling.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
