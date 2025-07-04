//! This module defines the `Extension` enum representing file extensions (json, yaml)
//! and provides conversions to and from strings, as well as mappings to `Format`.

use std::fmt::{self, Display};
use std::str::FromStr;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::serialization::format::Format;

/// Enumeration of supported file extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Extension {
    /// JSON file extension.
    #[default]
    Json,
    /// YAML file extension.
    Yaml,
}

impl Extension {
    /// Returns the extension as a string without a leading dot.
    ///
    /// # Examples
    /// ```
    /// assert_eq!(Extension::Json.as_str(), "json");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Json => "json",
            Extension::Yaml => "yaml",
        }
    }
}

/// Conversion from `Format` to `Extension`.
impl From<Format> for Extension {
    /// Converts an `Format` into a `Extension`.
    ///
    /// # Examples
    /// ```
    /// let format: Format = Extension::Json.into();
    /// ```
    fn from(format: Format) -> Self {
        match format {
            Format::Json => Extension::Json,
            Format::Yaml => Extension::Yaml,
        }
    }
}

/// Conversion from string to `Extension`.
impl FromStr for Extension {
    type Err = ParserInternalError;

    /// Parses a string (with or without leading dot) into an `Extension`.
    ///
    /// Supported values (case sensitive):
    /// - "json" or ".json"
    /// - "yaml", "yml", ".yaml", ".yml"
    ///
    /// # Errors
    /// Returns `ParserInternalError` if the extension is unknown.
    ///
    /// # Examples
    /// ```
    /// let ext = Extension::from_str("json")?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.');
        match normalized {
            "json" => Ok(Extension::Json),
            "yaml" | "yml" => Ok(Extension::Yaml),
            other => Err(ParserInternalError::new(format!("Unknown extension: {}", other))),
        }
    }
}

/// Allows displaying the extension as a string.
impl Display for Extension {
    /// Formats the extension for display (without leading dot).
    ///
    /// # Examples
    /// ```
    /// let s = Extension::Yaml.to_string();
    /// assert_eq!(s, "yaml");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
