//! Module `origin`.
//!
//! This module defines the [`Origin`] enum which represents the source or provenance
//! of a symbol in the context of PDDL (Planning Domain Definition Language) parsing and analysis.
//!
//! Symbols can originate from different parts of a PDDL problem definition:
//! - The domain file, which contains the general planning constructs (types, predicates, actions).
//! - The problem file, which specifies a particular planning instance (objects, initial state, goal).
//! - Or the origin might be unknown, typically due to incomplete parsing or errors.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Display labels associated with each variant of [`Origin`].
pub const SYMBOL_ORIGIN_DOMAIN: &str = "Domain";
pub const SYMBOL_ORIGIN_PROBLEM: &str = "Problem";
pub const SYMBOL_ORIGIN_UNKNOWN: &str = "Unknown";

/// Describes the origin of a symbol in a PDDL context.
///
/// This enum helps to track the provenance of symbols to improve
/// error reporting, validation, and semantic analysis.
///
/// # Variants
///
/// - `Domain`: Symbol originates from the domain file.
/// - `Problem`: Symbol originates from the problem file.
/// - `Unknown`: Origin is unknown or undetermined (default).
///
/// # Example
///
/// ```rust
/// use aiplan4rust::symbol::origin::Origin;
///
/// let origin = Origin::Domain;
/// assert_eq!(origin.to_string(), "Domain");
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Origin {
    /// Symbol is defined in the domain file.
    Domain,

    /// Symbol is defined in the problem file.
    Problem,

    /// Symbol origin is unknown or undetermined.
    #[default]
    Unknown,
}

impl fmt::Display for Origin {
    /// Formats the `Origin` as a human-readable string.
    ///
    /// Uses predefined string constants for consistent display labels.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Origin::Domain => SYMBOL_ORIGIN_DOMAIN,
            Origin::Problem => SYMBOL_ORIGIN_PROBLEM,
            Origin::Unknown => SYMBOL_ORIGIN_UNKNOWN,
        };
        write!(f, "{}", label)
    }
}
