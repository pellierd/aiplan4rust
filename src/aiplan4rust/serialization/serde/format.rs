use std::fmt;
use std::str::FromStr;

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::serialization::serde::SerdeExtension;

/// Represents supported serialization formats (JSON, YAML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Format {
    /// JSON format (default)
    #[default]
    Json,
    /// YAML format
    Yaml,
    /// TOML format
    Toml,
    /// CBOR format (binary, encoded as hex string for textual representation)
    Cbor,
    /// MessagePack format (binary, encoded as hex string for textual representation)
    MessagePack,
}

impl Format {
    /// Returns the format name as a lowercase string (e.g., `"json"`).
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(Format::Yaml.as_str(), "yaml");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
            Format::Cbor => "cbor",
            Format::MessagePack => "messagepack",
        }
    }
}

/// Converts a file extension into a `Format`.
impl From<SerdeExtension> for Format {
    /// Maps common file extensions to serialization formats.
    ///
    /// # Examples
    ///
    /// ```
    /// let format: Format = Extension::Json.into();
    /// ```
    fn from(ext: SerdeExtension) -> Self {
        match ext {
            SerdeExtension::Json => Format::Json,
            SerdeExtension::Yaml => Format::Yaml,
            SerdeExtension::Toml => Format::Toml,
            SerdeExtension::Cbor => Format::Cbor,
            SerdeExtension::MessagePack => Format::MessagePack,
            // Add other extensions if needed
        }
    }
}

/// Parses a `Format` from a string (case-insensitive, with or without leading dot).
impl FromStr for Format {
    type Err = AiplanError;

    /// Parses a string into a `Format`.
    ///
    /// # Errors
    ///
    /// Returns an error if the format string is unknown.
    ///
    /// # Examples
    ///
    /// ```
    /// let fmt = Format::from_str("yaml")?;
    /// let fmt_dot = Format::from_str(".json")?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim_start_matches('.').to_ascii_lowercase();
        match normalized.as_str() {
            "json" => Ok(Format::Json),
            "yaml" | "yml" => Ok(Format::Yaml),
            "toml" => Ok(Format::Toml),
            "cbor" => Ok(Format::Cbor),
            "messagepack" | "msgpack" => Ok(Format::MessagePack),
            other => Err(AiplanError::new(format!("Unknown format: {}", other))),
        }
    }
}

/// Implements Display for `Format`, returns the format name as string.
impl fmt::Display for Format {
    /// Formats the `Format` as a lowercase string.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = Format::Json.to_string();
    /// assert_eq!(s, "json");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
