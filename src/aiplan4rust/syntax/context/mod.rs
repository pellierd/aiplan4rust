//! This module provides the core parsing context and its associated error type_checker
//! used during the construction of the abstract syntax tree (AST).
//!
//! It exposes:
//! - [`ParseContext`]: A structure managing AST allocation, string interning,
//!   and error collection during parsing.
//! - [`ParseContextError`]: An error type_checker representing possible failures during
//!   context operations (e.g., arena errors or internal invariants).
//!
//! These components are intended for use by the LALRPOP-generated parser
//! and other modules responsible for AST construction and transformation.

pub mod context;
pub mod error;

pub use context::ParseContext;
pub use error::ParseContextError;
