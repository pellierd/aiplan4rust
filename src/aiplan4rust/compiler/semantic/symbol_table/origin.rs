//! Defines the `Origin` enum for representing the provenance of a symbol table.
//!
//! A symbol table's `Origin` indicates whether its entries came from a domain file,
//! a problem file, or a combination of both. This is useful for tooling, diagnostics,
//! and semantic validation stages, allowing contextual understanding of symbol sources.
//!
//! The `Origin` can also be converted into a `SymbolOrigin`, which is used
//! on a per-symbol basis.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::aiplan4rust::compiler::semantic::symbol::SymbolOrigin;

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
/// - `Unknown`: Default or unspecified origin; used as a placeholder.
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
    /// Symbol table built from a domain file.
    Domain,

    /// Symbol table built from a problem file.
    Problem,

    /// Unknown or unspecified origin.
    Unknown,
}

impl Default for Origin {
    /// Returns the debug value for `Origin`.
    ///
    /// The debug origin is `Unknown`, indicating an unspecified or placeholder origin.
    fn default() -> Self {
        Origin::Unknown
    }
}

impl fmt::Display for Origin {
    /// Formats the `Origin` as a human-readable string.
    ///
    /// The output is:
    /// - `"domain"` for `Origin::Domain`
    /// - `"problem"` for `Origin::Problem`
    /// - `"unspecified"` for `Origin::Unknown`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Origin::Domain => "domain",
            Origin::Problem => "problem",
            Origin::Unknown => "unknown",
        };
        write!(f, "{label}")
    }
}

/// Converts an `Origin` (table-level) to a `SymbolOrigin` (per-symbol granularity).
///
/// # Mapping
///
/// - `Origin::Domain` → `SymbolOrigin::Domain`
/// - `Origin::Problem` → `SymbolOrigin::Problem`
/// - `Origin::Unknown` → `SymbolOrigin::Unknown`
///
/// The `Merged` variant maps to `Unknown` because merged tables contain symbols from
/// multiple origins, and cannot be mapped to a single `SymbolOrigin`.
impl From<Origin> for SymbolOrigin {
    fn from(origin: Origin) -> Self {
        match origin {
            Origin::Domain => SymbolOrigin::Domain,
            Origin::Problem => SymbolOrigin::Problem,
            Origin::Unknown => SymbolOrigin::Unknown,
        }
    }
}
