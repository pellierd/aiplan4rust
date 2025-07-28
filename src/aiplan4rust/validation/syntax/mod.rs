//! Validation utilities for AST correctness and well-formedness checks.
//!
//! This module provides functions to validate the structural integrity and
//! correctness of Abstract Syntax Tree (AST) nodes within the system.
//!
//! It includes:
//! - `check_well_formed`: Validates the entire AST or a subtree for well-formedness.
//! - `check_well_formed_node`: Checks an individual AST node for correctness.
//! - `is_well_formed`: Returns a boolean indicating if a node or AST satisfies well-formedness criteria.
//!
//! These utilities leverage the submodule [`checks`] for detailed validation logic.
//!
//! # Examples
//!
//! ```rust
//! use your_crate::validation::{check_well_formed, is_well_formed};
//!
//! if let Err(e) = check_well_formed(&ast) {
//!     eprintln!("AST is not well-formed: {}", e);
//! }
//!
//! if is_well_formed(&node) {
//!     println!("Node is well-formed");
//! } else {
//!     println!("Node is malformed");
//! }
//! ```
pub mod validation;
pub mod checks;

pub use validation::check_well_formed;
pub use validation::check_well_formed_node;
pub use validation::is_well_formed;
