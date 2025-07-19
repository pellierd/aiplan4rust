//! This module defines the `Extension` enum representing supported planning language
//! file extensions and provides conversions to/from strings as well as mappings
//! to the corresponding `PlanningFormat`.
//!
//! Supported extensions include:
//! - `.pddl` for PDDL files (default)
//! - `.hddl` for HDDL files

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::serialization::planning::PlanningFormat;

/// Enumeration of supported planning language file extensions.
///
/// # Variants
///
/// - `Pddl`: Represents `.pddl` files (default).
/// - `Hddl`: Represents `.hddl` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Extension {
    /// PDDL file extension (default).
    #[default]
    Pddl,
    /// HDDL file extension.
    Hddl,
}

impl Extension {
    /// Returns the extension as a lowercase string without the leading dot.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(Extension::Pddl.as_str(), "pddl");
    /// assert_eq!(Extension::Hddl.as_str(), "hddl");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Pddl => "pddl",
            Extension::Hddl => "hddl",
        }
    }
}

/// Converts a `PlanningFormat` into the corresponding `Extension`.
///
/// # Examples
///
/// ```
/// let ext: Extension = PlanningFormat::Pddl.into();
/// assert_eq!(ext.as_str(), "pddl");
/// ```
impl From<PlanningFormat> for Extension {
    fn from(format: PlanningFormat) -> Self {
        match format {
            PlanningFormat::Pddl => Extension::Pddl,
            PlanningFormat::Hddl => Extension::Hddl,
        }
    }
}

/// Parses a string slice into a `Extension`.
///
/// The input string may optionally start with a dot (`.`) and is case-insensitive.
///
/// Supported values:
/// - `"pddl"` or `".pddl"`
/// - `"hddl"` or `".hddl"`
///
/// # Errors
///
/// Returns a `ParserInternalError` if the input does not match any supported extension.
///
/// # Examples
///
/// ```
/// let ext = Extension::from_str(".pddl")?;
/// assert_eq!(ext, Extension::Pddl);
/// ```
impl FromStr for Extension {
    type Err = AiplanError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.').to_ascii_lowercase();
        match normalized.as_str() {
            "pddl" => Ok(Extension::Pddl),
            "hddl" => Ok(Extension::Hddl),
            other => Err(AiplanError::new(format!("Unknown planning extension: {}", other))),
        }
    }
}

/// Allows displaying the extension as a string.
impl Display for Extension {
    /// Formats the extension for display (without leading dot).
    ///
    /// # Examples
    ///
    /// ```
    /// let s = Extension::Yaml.to_string();
    /// assert_eq!(s, "yaml");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
