//! Provides utilities and checks for validating the correctness and normalization of AST nodes.
//!
//! This module exposes two submodules:
//! - [`checks`]: Contains functions to perform various validation checks on AST nodes.
//! - [`validation`]: Contains functions focused on verifying that AST nodes are well normalized.
//!
//! The main validation functions are re-exported for easier access:
//! - [`check_well_normalized`]: Validates that an entire AST or subtree is well normalized.
//! - [`check_well_normalized_node`]: Validates that a single AST node is well normalized.
//! - [`is_well_normalized`]: Returns a boolean indicating whether a node or AST is well normalized.
//!
//! # Usage
//!
//! Use the functions in [`checks`] to perform generic validation checks,
//! and use those in [`validation`] to assert normalization properties of AST nodes.
//!
//! # Examples
//!
//! ```rust
//! use your_crate::validation_module::{check_well_normalized, is_well_normalized};
//!
//! // Assuming you have an AST root node `root`
//! if let Err(e) = check_well_normalized(&root) {
//!     eprintln!("AST is not well normalized: {}", e);
//! }
//!
//! let normalized = is_well_normalized(&root);
//! println!("Is AST well normalized? {}", normalized);
//! ```
pub mod checks;
pub mod validation;

pub use validation::check_well_normalized;
pub use validation::check_well_normalized_node;
pub use validation::is_well_normalized;
