use serde::{Deserialize, Serialize};
use std::fmt;

/// Constants representing the display values for each source variant.
pub const SOURCE_DOMAIN: &str = "Domain";
pub const SOURCE_PROBLEM: &str = "Problem";
pub const SOURCE_UNKNOWN: &str = "Unknown";

/// Represents the origin of a symbol within a PDDL file.
///
/// Symbols may originate from either the domain file, the problem file,
/// or an unknown context (e.g., due to incomplete parsing).
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Source {
    /// Symbol originates from the domain file.
    #[default]
    Domain,

    /// Symbol originates from the problem file.
    Problem,

    /// Symbol's origin is unknown.
    Unknown,
}

impl fmt::Display for Source {
    /// Formats the `Source` enum into a human-readable string.
    ///
    /// Uses constants instead of hardcoded string literals.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Source::Domain => SOURCE_DOMAIN,
            Source::Problem => SOURCE_PROBLEM,
            Source::Unknown => SOURCE_UNKNOWN,
        };
        write!(f, "{}", label)
    }
}
