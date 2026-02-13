//! Type checking infrastructure for the AIPLan4Rust semantic analyzer.
//!
//! This module exposes the main components involved in the **type checking phase**
//! of semantic analysis. It is responsible for validating types across the abstract syntax tree (AST),
//! ensuring correct usage of symbols, expr, and declarations according to the language's rules.
//!
//! # Modules
//!
//! - [`type_checker`] — Contains the common logic of the [`TypeChecker`] struct, which traverses
//!   the AST and performs type validation.
//! - [`error`] — Defines [`TypeCheckError`], the error type used to report issues during type checking.
//!
//! # Re-exports
//!
//! - [`TypeChecker`] — The entry point for invoking type checking on an AST.
//! - [`TypeCheckError`] — The unified error type for type checking failures.
//!
//! # Example
//!
//! ```rust,no_run
//! use aiplan4rust::semantic::type_check::{TypeChecker, TypeCheckError};
//! use aiplan4rust::syntax::ast::Ast;
//!
//! fn perform_type_check(ast: &Ast) -> Result<(), TypeCheckError> {
//!     let mut checker = TypeChecker::new();
//!     checker.check(ast)?;
//!     Ok(())
//! }
//! ```
//!
//! # See Also
//!
//! - [`SymbolTable`](crate::aiplan4rust::semantic::SymbolTable)
//! - [`Type`](crate::aiplan4rust::lang::Type)
//! - [`SymbolRef`](crate::aiplan4rust::syntax::symbol::SymbolRef)
//!
//! This module is part of the semantic layer and assumes a correctly constructed symbol table.

pub mod type_checker;
pub mod error;

pub use error::TypeCheckError;
