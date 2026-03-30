//! This module organizes and exposes semantic consistency checks
//! focused on ensuring coherence between symbols declared in the domain
//! and those referenced in the problem within a PDDL analysis context.
//!
//! It includes two primary submodules:
//!
//! - `domain_name`: Contains functions to verify that the domain name declared
//!   in the domain file matches the domain name referenced in the problem file.
//!
//! - `cross_declared_symbols`: Provides functions to detect and report
//!   conflicting declarations between symbols present in the domain and those
//!   declared in the problem, helping to ensure semantic consistency.
//!
//! Additionally, this module re-exports key checking functions and the
//! `LinkingCheckError` typing at its root for convenient access during
//! linking and semantic analysis phases.

/// Module for verifying that the declared domain name matches
/// the domain name referenced in the problem.
pub mod domain_name;

/// Module for detecting and reporting symbol declaration conflicts
/// between the problem and the domain.
pub mod cross_declared_symbols;

/// Module containing error types related to linking checks.
pub mod error;
mod link;
mod unresolved_usage;

pub use cross_declared_symbols::check_cross_declared_symbols;
pub use domain_name::check_domain_name;
pub use error::LinkingCheckError;
pub use link::perform_linking;
pub use unresolved_usage::check_unresolved_usages;
