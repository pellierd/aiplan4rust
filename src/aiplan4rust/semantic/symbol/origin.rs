use serde::{Deserialize, Serialize};
use std::fmt;

/// Display labels associated with each variant of `Origine`.
pub const SYMBOL_SOURCE_DOMAIN: &str = "Domain";
pub const SYMBOL_SOURCE_PROBLEM: &str = "Problem";
pub const SYMBOL_SOURCE_UNKNOWN: &str = "Unknown";

/// Describes the origin of a symbol in a PDDL (Planning Domain Definition Language) context.
///
/// Symbols used in PDDL planning problems may come from different sources:
/// - The domain file, which defines general planning constructs (e.g., types, predicates, actions).
/// - The problem file, which defines a specific planning instance (e.g., objects, initial state,
///   goal).
/// - Or an unknown context, typically resulting from parsing failures or incomplete input.
///
/// This enum helps track the provenance of each symbol to assist in error reporting,
/// validation, and analysis.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::symbol::origin::Origine;
///
/// let origin = Origine::Domain;
/// assert_eq!(origin.to_string(), "Domain");
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Origin {
    /// The symbol originates from the domain file.
    Domain,

    /// The symbol originates from the problem file.
    Problem,

    /// The symbol's origin could not be determined (default).
    #[default]
    Unknown,
}

impl fmt::Display for Origin {
    /// Converts the `Origine` variant to a human-readable string.
    ///
    /// This implementation uses predefined constants for consistency and easy maintenance.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Origin::Domain => SYMBOL_SOURCE_DOMAIN,
            Origin::Problem => SYMBOL_SOURCE_PROBLEM,
            Origin::Unknown => SYMBOL_SOURCE_UNKNOWN,
        };
        write!(f, "{}", label)
    }
}
