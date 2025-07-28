//! Module grouping functionalities related to string interning.
//!
//! This module exposes several submodules and main types to manage string interners,
//! handle specific errors, display interners, and merge multiple interners.
//!
//! # Submodules
//!
//! - [`interner`]: Contains the core implementation of string interning via `StringInterner`.
//! - [`merged_result`]: Provides structures and functions to merge multiple interners using `InternerMergeResult`.
//! - [`display`]: Offers tools to display or format interners (`InternerDisplay`).
//! - [`error`]: Defines errors specific to interning (`InternerError`).
//!
//! # Exposed types
//!
//! - [`StringInterner`]: The main string interner implementation.
//! - [`InternerMergeResult`]: Result type_checker for interner merge operations.
//! - [`InternerDisplay`]: Utilities for displaying/formatting interners.
//! - [`InternerError`]: Enum representing errors related to interning.
//!
//! # Example
//!
//! ```rust
//! use your_crate::StringInterner;
//!
//! let mut interner = StringInterner::new();
//! let id = interner.intern("example".to_string());
//! println!("Interned string id: {:?}", id);
//! ```
pub mod interner;
pub mod merged_result;
pub mod display;
pub mod error;
pub mod ident;
pub mod literal;
pub mod id;

pub use interner::StringInterner;
pub use merged_result::InternerMergeResult;
pub use display::InternerDisplay;
pub use error::InternerError;
pub use id::Id as InternerId;
pub use literal::Literal;
pub use ident::Ident;
