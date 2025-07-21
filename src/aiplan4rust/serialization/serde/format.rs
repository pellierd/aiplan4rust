use std::fmt;
use std::str::FromStr;
use clap::ValueEnum;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::serialization::serde::SerdeExtension;

/// Represents supported serialization formats (JSON, YAML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, ValueEnum)]
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
    /// Maps core file extensions to serialization formats.
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

impl FromStr for Format {
    type Err = AiplanError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as clap::ValueEnum>::from_str(s, false)
            .map_err(|e| AiplanError::InternalError(format!("Invalid format: {}", e)))
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
