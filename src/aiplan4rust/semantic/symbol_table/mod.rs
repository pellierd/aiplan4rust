//! Defines the symbol table system used in semantic analysis.
//!
//! A symbol table manages the association between identifiers (names) and their meaning,
//! such as types, declarations, or references in the AST. This module provides a clean
//! interface for building, querying, and debugging symbol tables across both domain
//! and problem inputs.
//!
//! # Structure
//!
//! - [`builder`] — Facilities to construct symbol tables from an AST.
//! - [`table`] — Core definition of the [`SymbolTable`] structure, which stores and resolves symbols.
//! - [`origin`] — Tracks the provenance of a symbol table, e.g., domain/problem/merged.
//! - [`error`] — Custom error types used throughout symbol table logic.
//!
//! # Re-exports
//!
//! - [`SymbolTable`] is the main interface to query and interact with symbols.
//! - [`SymbolTableError`] groups all possible errors from table building/resolution.
//! - [`SymbolTableOrigin`] identifies the high-level source of a table (domain, problem, etc.).
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::semantic::symbol_table::{SymbolTable, SymbolTableOrigin};
//!
//! let origin = SymbolTableOrigin::Domain;
//! // SymbolTable instances are typically created via a builder from the AST.
//! ```
//!
//! Internal usage (not public API):
//! - `SymbolTableBuilder` is exposed internally for constructing symbol tables.

pub mod table;
pub mod builder;
pub mod origin;
pub mod error;

/// Main symbol table interface for semantic resolution.
pub use table::Table as SymbolTable;

/// Public error type for symbol table construction/resolution.
pub use error::SymbolTableError;

/// Describes the origin of the symbol table (domain/problem/merged).
pub use origin::Origin as SymbolTableOrigin;

/// Internal builder used to construct symbol tables from AST.
/// Not part of the public API.
pub(crate) use builder::SymbolTableBuilder;
