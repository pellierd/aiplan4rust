//! Symbol management module for the AI syntax Rust framework.
//!
//! This module provides core components and abstractions for handling symbols in the
//! language's semantic analysis and AST representation.
//!
//! # Submodules
//!
//! - [`scope`]: Defines symbol scopes (e.g., global, local) controlling visibility and lifetime.
//! - [`usage`]: Tracks and represents symbol usages throughout the program.
//! - [`declaration`]: Represents symbol declarations, capturing identity, scope, types, and source location.
//! - [`filterable`]: Utilities to filter collections of symbols or declarations based on criteria.
//! - [`entry`]: Symbol table entries, organizing symbol metadata and storage.
//! - [`kind`]: Enumerates kinds of symbols (variables, functions, constants, etc.).
//! - [`origin`]: Tracks the origin/source of symbols (e.g., domain vs problem symbols).
//! - [`symbol`]: Core symbol types and references.
//!
//! # Re-exports
//!
//! For convenience, key types from submodules are re-exported here:
//! - [`Declaration`] from the `declaration` module.
//! - [`Filterable`] trait for filtering collections.
//! - [`Scope`] from the `scope` module.
//! - [`SymbolEntry`] from the `entry` module.
//! - [`SymbolKind`] (aliased from `kind::Kind`).
//! - [`Usage`] from the `usage` module.
//! - [`SymbolOrigin`] (aliased from `origin::Origin`).
//! - [`SymbolRef`] (aliased from `symbol::Symbol`).
//!
//! These types form the foundation for working with symbols in the framework,
//! enabling declaration, usage tracking, and semantic analysis.

pub mod scope;
pub mod usage;

pub mod declaration;
pub mod filterable;
pub mod entry;
pub mod kind;
pub mod origin;
pub mod symbol;

pub use declaration::Declaration;
pub use filterable::Filterable;
pub use scope::Scope;
pub use entry::SymbolEntry;
pub use kind::Kind as SymbolKind;
pub use usage::Usage;
pub use origin::Origin as SymbolOrigin;
pub use symbol::Symbol;
