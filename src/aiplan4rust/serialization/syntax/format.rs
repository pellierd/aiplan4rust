//! This module defines the `Format` enum, representing supported syntax serialization formats
//! for syntax languages, specifically HDDL and PDDL.
//!
//! It includes functionality to:
//! - Convert from file extensions (`PlanningExtension`) to `Format`,
//! - Parse formats from strings (case-insensitive, with or without leading dots),
//! - Format `Format` variants as lowercase strings for display.
//!
//! # Supported formats
//! - HDDL (default)
//! - PDDL
//!
//! # Usage examples
//!
//! ```
//! use std::str::FromStr;
//! use your_crate::Format;
//!
//! let f = Format::from_str("hddl").unwrap();
//! assert_eq!(f.to_string(), "hddl");
//!
//! let g = Format::from_str(".pddl").unwrap();
//! assert_eq!(g.to_string(), "pddl");
//! ```

use std::fmt;
use std::str::FromStr;

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::serialization::syntax::PlanningExtension; // Adjust if you have a specific Extension enum for syntax

/// This module defines the `Format` enum representing supported syntax serialization formats,
/// specifically for syntax languages such as HDDL and PDDL.
///
/// It provides conversions between file extensions and formats,
/// parsing from strings, and string display implementations.
///
/// # Supported formats
///
/// - HDDL (default)
/// - PDDL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Format {
    /// PDDL format
    Pddl,
    /// HDDL format (default)
    #[default]
    Hddl,
}

impl Format {
    /// Returns the format name as a lowercase string.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(Format::Hddl.as_str(), "hddl");
    /// assert_eq!(Format::Pddl.as_str(), "pddl");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Hddl => "hddl",
            Format::Pddl => "pddl",
        }
    }
}

/// Converts a file extension (`PlanningExtension`) into a `Format`.
///
/// # Arguments
///
/// * `ext` - The file extension to convert.
///
/// # Returns
///
/// Corresponding `Format` variant. If the extension is unknown,
/// returns `Format::Hddl` by default (you may customize this behavior).
impl From<PlanningExtension> for Format {
    fn from(ext: PlanningExtension) -> Self {
        match ext {
            PlanningExtension::Hddl => Format::Hddl,
            PlanningExtension::Pddl => Format::Pddl,
            // Add more extensions here if necessary
        }
    }
}

/// Parses a `Format` from a string slice, case-insensitive, with or without a leading dot.
///
/// # Arguments
///
/// * `s` - String slice representing the format or file extension.
///
/// # Errors
///
/// Returns a `ParserInternalError` if the string does not correspond to a known format.
///
/// # Examples
///
/// ```
/// let f = Format::from_str("hddl")?;
/// let g = Format::from_str(".pddl")?;
/// ```
impl FromStr for Format {
    type Err = AiplanError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.').to_ascii_lowercase();
        match normalized.as_str() {
            "hddl" => Ok(Format::Hddl),
            "pddl" => Ok(Format::Pddl),
            other => Err(AiplanError::InternalError(format!("Unknown syntax format: {}", other))),
        }
    }
}

/// Implements the `Display` trait to allow formatting a `Format` as a string.
///
/// The output is the lowercase string representation of the format,
/// without any leading dots.
///
/// # Examples
///
/// ```
/// let s = Format::Hddl.to_string();
/// assert_eq!(s, "hddl");
/// ```
impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
