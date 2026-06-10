//! Error types related to typing checking in the semantic analysis phase.
//!
//! This module defines the [`TypeCheckerError`] enum, which captures all errors
//! that can arise during the typing checking process of an abstract syntax tree (AST).
//!
//! Type checking is a crucial part of semantic analysis, responsible for ensuring
//! that operations, logic, and declarations conform to the language’s typing rules.
//! During this process, a number of errors can occur — from internal ops inconsistencies
//! to invalid or missing symbol declarations.
//!
//! # Contents
//!
//! - [`TypeCheckerError`] — the main error typing for reporting issues during typing checking.
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
//!     Err(TypeCheckError::internal_error("unexpected typing found"))
//! }
//! ```
//!
//! # See Also
//!
//! - [`SymbolTableError`](crate::aiplan4rust::compiler::semantic::symbol_table::SymbolTableError)
//! - [`Type`](crate::aiplan4rust::support::lang::Type)
//! - [`SymbolTable`](crate::aiplan4rust::compiler::semantic::SymbolTable)
//!
//! [`thiserror`]: https://docs.rs/thiserror

use crate::aiplan4rust::compiler::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// Represents errors that may occur during the typing checking phase of semantic analysis.
///
/// `TypeCheckError` encapsulates all possible failure modes that can arise when
/// performing typing checking over an abstract syntax tree (AST). It includes:
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
/// - [`SymbolTable`]: A wrapper for [`SymbolTableError`], allowing typing check
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
/// [`SymbolTableError`]: crate::aiplan4rust::compiler::semantic::symbol_table::SymbolTableError
/// [`thiserror`]: https://docs.rs/thiserror
#[derive(Debug, Error)]
pub enum TypeCheckerError {
    /// Error originating from the symbol table layer.
    ///
    /// Allows symbol table construction errors to be transparently surfaced during typing checking.
    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    /// Indicates that a type union (`either`) contains too many types to be processed
    /// by the optimized bitmask-based simplification (limit is 64).
    ///
    /// This is an extremely rare case in PDDL domains but acts as a safety guard
    /// for the underlying bitwise operations.
    #[error("Type union capacity exceeded: {0} members found, but a maximum of 64 is supported for simplification")]
    TypeUnionCapacityExceeded(usize),
}

impl TypeCheckerError {
    /// Creates a new `TypeUnionCapacityExceeded` error.
    ///
    /// This error is raised when a type union (e.g., an `either` declaration)
    /// contains more than 64 primitive types. This limit is imposed by the
    /// optimized bitmask-based simplification algorithm.
    ///
    /// # Arguments
    ///
    /// * `count` - The actual number of members found in the type union.
    ///
    /// # Returns
    ///
    /// A `TypeCheckError` variant specifically for capacity overflow.
    pub fn type_union_capacity_exceeded(count: usize) -> Self {
        Self::TypeUnionCapacityExceeded(count).trace()
    }
}

impl Traceable for TypeCheckerError {}
