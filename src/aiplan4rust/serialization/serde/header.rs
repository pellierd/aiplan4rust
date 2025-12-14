//! Module defining file headers for serialized objects.
//!
//! The header allows identification of files produced by our application,
//! includes a magic number, versioning, format, and generation timestamp.

use std::fmt::Display;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::aiplan4rust::serialization::serde::format::Format;
use crate::aiplan4rust::serialization::serde::SerdeFormat;

/// Fixed-size magic number for identifying files produced by the application.
pub const MAGIC: &[u8; 4] = b"AIPL";

/// Represents the header for a serialized file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Magic number identifying the file.
    pub magic: [u8; 4],

    /// Version of the header structure.
    pub version: u8,

    /// Serialization format of the payload.
    pub format: SerdeFormat,

    /// ISO 8601 timestamp indicating when the file was generated.
    pub generated_at: String,
}
impl Header {
    /// Creates a new `Header` with the specified serialization format and version.
    ///
    /// The header will include:
    /// - A fixed magic number (`b"AIPL"`) identifying files produced by the application.
    /// - The provided `version` number.
    /// - The specified serialization `format`.
    /// - A `generated_at` timestamp in ISO 8601 format (UTC) representing the creation time.
    ///
    /// # Arguments
    ///
    /// * `format` - The serialization format of the payload (`SerdeFormat`).
    /// * `version` - The version number of the header structure.
    ///
    /// # Returns
    ///
    /// Returns a `Header` instance initialized with the given format and version, and the current timestamp.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::aiplan4rust::serialization::serde::{Header, SerdeFormat};
    ///
    /// let header = Header::new(SerdeFormat::Json, 1);
    /// assert_eq!(header.version, 1);
    /// assert_eq!(header.format, SerdeFormat::Json);
    /// assert_eq!(&header.magic, b"AIPL");
    /// ```
    pub fn new(format: Format, version: u8) -> Self {
        Self {
            magic: *MAGIC,
            version,
            format,
            generated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Validates that the header's magic number matches the expected constant.
    ///
    /// This can be used to verify that a file was produced by this application
    /// and is likely a valid serialized file.
    ///
    /// # Returns
    ///
    /// `true` if the `magic` field equals `b"AIPL"`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::aiplan4rust::serialization::serde::Header;
    ///
    /// let header = Header::new(crate::aiplan4rust::serialization::serde::SerdeFormat::Json, 1);
    /// assert!(header.validate_magic());
    /// ```
    pub fn validate_magic(&self) -> bool {
        &self.magic == MAGIC
    }
}

/// Implements a user-friendly display for the `Header` struct.
///
/// The `Display` implementation prints the header in a multiline format:
/// - Tries to render the `magic` bytes as a UTF-8 string.
/// - If UTF-8 decoding fails, it prints the bytes in hexadecimal.
/// - Prints the `version`, `format`, and `generated_at` fields with indentation.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::aiplan4rust::serialization::serde::header::Header;
/// use aiplan4rust::aiplan4rust::serialization::serde::SerdeFormat;
/// use crate::aiplan4rust::serialization::serde::header::{Header, MAGIC};
/// use crate::aiplan4rust::serialization::serde::SerdeFormat;
///
/// let header = Header::new(SerdeFormat::Json, 1);
/// println!("{}", header);
/// ```
///
/// Possible output:
///
/// ```text
/// Header {
///     magic       : AIPL
///     version     : 1
///     format      : json
///     generated_at: 2025-12-14T10:42:00Z
/// }
/// ```
///
/// If the magic bytes are not valid UTF-8:
///
/// ```text
/// Header {
///     magic       : 41 49 50 4C
///     version     : 1
///     format      : json
///     generated_at: 2025-12-14T10:42:00Z
/// }
/// ```
impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let magic_str = std::str::from_utf8(&self.magic)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| {
                self.magic.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
            });

        writeln!(f, "Header {{")?;
        writeln!(f, "    magic       : {}", magic_str)?;
        writeln!(f, "    version     : {}", self.version)?;
        writeln!(f, "    format      : {}", self.format)?;
        writeln!(f, "    generated_at: {}", self.generated_at)?;
        write!(f, "}}")
    }
}
