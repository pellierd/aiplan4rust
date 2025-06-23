use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::semantic::symbol::SymbolOrigin;

/// Represents the origin of a symbol table, indicating the provenance of its symbols.
///
/// This enum is used to track where a symbol table's entries were sourced from,
/// typically corresponding to distinct AST inputs or combined/linking phases.
///
/// It is useful for diagnostics, error reporting, and tooling to understand symbol provenance.
///
/// # Variants
///
/// - `Domain`: The table was constructed exclusively from the domain AST.
/// - `Problem`: The table was constructed exclusively from the problem AST.
/// - `Merged`: The table results from linking/merging domain and problem symbols.
///   Note that individual symbols within may have a more specific origin.
/// - `Unknown`: Default or unspecified origin; used as a placeholder.
///
/// # Conversion to `SymbolOrigin`
///
/// This enum can be converted into a `SymbolOrigin` (which marks
/// individual symbols) via the `From` trait. Note that the `Merged` variant
/// maps to `SymbolOrigin::Unknown`, as merged tables don't correspond to
/// a single symbol origin.
///
/// # Example
///
/// ```rust
/// use your_crate::Origin;
///
/// let origin = Origin::Domain;
/// assert_eq!(format!("{}", origin), "domain");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origin {
    /// Built from a domain file.
    Domain,

    /// Built from a problem file.
    Problem,

    /// Result of merging a domain and a problem symbol table.
    Merged,

    /// Unspecified or default origin.
    Unknown,
}

impl Default for Origin {
    /// Returns the default value for `Origin`.
    ///
    /// The default origin is `Unknown`, indicating an unspecified or placeholder origin.
    fn default() -> Self {
        Origin::Unknown
    }
}


impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Origin::Domain => "domain",
            Origin::Problem => "problem",
            Origin::Merged => "merged (domain + problem)",
            Origin::Unknown => "unspecified",
        };
        write!(f, "{label}")
    }
}

/// Converts a `SymbolTableOrigin` to a `SymbolOrigin` for individual symbols.
///
/// Note: `Merged` maps to `Unknown` since merged tables combine multiple origins.
impl From<Origin> for SymbolOrigin {
    fn from(origin: Origin) -> Self {
        match origin {
            Origin::Domain => SymbolOrigin::Domain,
            Origin::Problem => SymbolOrigin::Problem,
            Origin::Merged | Origin::Unknown => SymbolOrigin::Unknown,
        }
    }
}
