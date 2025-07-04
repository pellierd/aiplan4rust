use std::fmt::{self, Display};
use std::str::FromStr;

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::serialization::Extension;

/// Represents supported serialization formats (JSON, YAML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Format {
    /// JSON format
    #[default]
    Json,
    /// YAML format
    Yaml,
}

impl Format {
    /// Returns the format name as a string (e.g., `"json"`).
    ///
    /// # Examples
    /// ```
    /// assert_eq!(Format::Yaml.as_str(), "yaml");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
        }
    }
}

/// Converts an `Extension` into a `Format`.
impl From<Extension> for Format {
    /// Maps file extensions directly to formats.
    ///
    /// # Examples
    /// ```
    /// let format: Format = Extension::Json.into();
    /// ```
    fn from(ext: Extension) -> Self {
        match ext {
            Extension::Json => Format::Json,
            Extension::Yaml => Format::Yaml,
        }
    }
}

/// Parses a `Format` from a string like `"json"`, `"yaml"` (with or without dot).
impl FromStr for Format {
    type Err = ParserInternalError;

    /// Converts a string (like `"json"` or `".yaml"`) to a `Format`.
    ///
    /// # Errors
    /// Returns an error if the format is unknown.
    ///
    /// # Examples
    /// ```
    /// let fmt = Format::from_str("yaml")?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.');
        match normalized {
            "json" => Ok(Format::Json),
            "yaml" | "yml" => Ok(Format::Yaml),
            other => Err(ParserInternalError::new(format!("Unknown format: {}", other))),
        }
    }
}

/// Display implementation for `Format`, returns the format name (e.g. `"json"`).
impl Display for Format {
    /// Formats the `Format` as a lowercase string.
    ///
    /// # Examples
    /// ```
    /// let s = Format::Json.to_string();
    /// assert_eq!(s, "json");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
