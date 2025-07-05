//! This module defines the `Extension` enum representing file extensions and
//! provides conversions to/from strings, as well as mappings to `Format`.

use std::fmt::{self, Display};
use std::str::FromStr;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::serialization::serde::SerdeFormat;

/// Enumeration of supported file extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Extension {
    /// JSON file extension.
    #[default]
    Json,
    /// YAML file extension.
    Yaml,
    /// TOML file extension.
    Toml,
    /// CBOR file extension.
    Cbor,
    /// MessagePack file extension.
    MessagePack,
}

impl Extension {
    /// Returns the extension as a string without a leading dot.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(Extension::Json.as_str(), "json");
    /// assert_eq!(Extension::MessagePack.as_str(), "msgpack");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Json => "json",
            Extension::Yaml => "yaml",
            Extension::Toml => "toml",
            Extension::Cbor => "cbor",
            Extension::MessagePack => "msgpack",
        }
    }
}

/// Conversion from `Format` to `Extension`.
impl From<SerdeFormat> for Extension {
    /// Converts a `Format` into an `Extension`.
    ///
    /// # Examples
    ///
    /// ```
    /// let extension: Extension = Format::Json.into();
    /// ```
    fn from(format: SerdeFormat) -> Self {
        match format {
            SerdeFormat::Json => Extension::Json,
            SerdeFormat::Yaml => Extension::Yaml,
            SerdeFormat::Toml => Extension::Toml,
            SerdeFormat::Cbor => Extension::Cbor,
            SerdeFormat::MessagePack => Extension::MessagePack,
        }
    }
}

/// Conversion from string to `Extension`.
impl FromStr for Extension {
    type Err = ParserInternalError;

    /// Parses a string (with or without leading dot) into an `Extension`.
    ///
    /// Supported values (case insensitive):
    /// - "json" or ".json"
    /// - "yaml", "yml", ".yaml", ".yml"
    /// - "toml", ".toml"
    /// - "cbor", ".cbor"
    /// - "msgpack", "messagepack", ".msgpack", ".messagepack"
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the extension is unknown.
    ///
    /// # Examples
    ///
    /// ```
    /// let ext = Extension::from_str("json")?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.').to_ascii_lowercase();
        match normalized.as_str() {
            "json" => Ok(Extension::Json),
            "yaml" | "yml" => Ok(Extension::Yaml),
            "toml" => Ok(Extension::Toml),
            "cbor" => Ok(Extension::Cbor),
            "msgpack" | "messagepack" => Ok(Extension::MessagePack),
            other => Err(ParserInternalError::new(format!("Unknown extension: {}", other))),
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
