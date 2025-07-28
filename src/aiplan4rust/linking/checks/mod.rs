//! This module organizes and exposes semantic consistency checks related to the coherence
//! between symbols declared in the domain and problem within a PDDL analysis context.
//!
//! It contains two main submodules:
//! - `domain_name`: provides functions to verify that the domain name declared in the domain file
//!   matches the domain name referenced in the problem file.
//! - `cross_declared_symbols`: provides functions to detect conflicting declarations between
//!   symbols present in the domain and those in the problem.
//!
//! This module also re-exports key checking functions at the root level for easier access
//! during linking and semantic analysis processes.
//
/// Module for verifying that the declared domain name matches the domain name referenced in the problem.
pub mod domain_name;

/// Module for detecting and reporting symbol declaration conflicts between the problem and domain.
pub mod cross_declared_symbols;

/// Re-exports the domain name consistency check function.
pub use domain_name::check_domain_name;

/// Re-exports the cross-declared symbol conflict detection function.
pub use cross_declared_symbols::check_cross_declared_symbols;
