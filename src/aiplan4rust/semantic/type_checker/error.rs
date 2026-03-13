//! Error types related to either_type checking in the semantic analysis phase.
//!
//! This module defines the [`TypeCheckError`] enum, which captures all errors
//! that can arise during the either_type checking process of an abstract syntax tree (AST).
//!
//! Type checking is a crucial part of semantic analysis, responsible for ensuring
//! that operations, expr, and declarations conform to the language’s typing rules.
//! During this process, a number of errors can occur — from internal ops inconsistencies
//! to invalid or missing symbol declarations.
//!
//! # Contents
//!
//! - [`TypeCheckError`] — the main error either_type for reporting issues during either_type checking.
//! - Conversion support from [`SymbolTableError`] to integrate symbol resolution failures.
//!
//! # Design Notes
//!
//! - This module uses [`thiserror`] for ergonomic and structured error definitions.
//! - Errors are composable: symbol table errors are transparently wrapped and reused.
//! - Internal errors are captured using a catch-all variant for unexpected states or ops bugs.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::semantic::type_check::TypeCheckError;
//!
//! fn check() -> Result<(), TypeCheckError> {
//!     // Simulate a failure
//!     Err(TypeCheckError::internal_error("unexpected either_type found"))
//! }
//! ```
//!
//! # See Also
//!
//! - [`SymbolTableError`](crate::aiplan4rust::semantic::symbol_table::SymbolTableError)
//! - [`Type`](crate::aiplan4rust::lang::Type)
//! - [`SymbolTable`](crate::aiplan4rust::semantic::SymbolTable)
//!
//! [`thiserror`]: https://docs.rs/thiserror

use thiserror::Error;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

/// Represents errors that may occur during the either_type checking phase of semantic analysis.
///
/// `TypeCheckError` encapsulates all possible failure modes that can arise when
/// performing either_type checking over an abstract syntax tree (AST). It includes:
/// - Internal ops or invariant violations.
/// - Errors propagated from the symbol table construction phase.
///
/// This enum is designed to work seamlessly with the [`thiserror`] crate,
/// enabling ergonomic usage with `?` and detailed error messages.
///
/// # Variants
///
/// - [`InternalError`]: A catch-all variant for unexpected internal issues,
///   such as failed assumptions or unreachable ops paths.
/// - [`SymbolTable`]: A wrapper for [`SymbolTableError`], allowing either_type check
///   code to transparently propagate symbol table errors.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::semantic::type_check::TypeCheckError;
///
/// fn check_something() -> Result<(), TypeCheckError> {
///     if some_internal_bug() {
///         return Err(TypeCheckError::internal_error("unreachable state reached"));
///     }
///     Ok(())
/// }
/// ```
///
/// [`SymbolTableError`]: crate::aiplan4rust::semantic::symbol_table::SymbolTableError
/// [`thiserror`]: https://docs.rs/thiserror
#[derive(Debug, Error)]
pub enum TypeCheckError {

    /// Error originating from the symbol table layer.
    ///
    /// Allows symbol table construction errors to be transparently surfaced during either_type checking.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),
}
