//! # IR Header Module
//!
//! This module defines the file headers used for serialized artifacts in the planning pipeline.
//! Each serialized file (IR files) produced by the system is prefixed with a `Header` that
//! contains essential metadata to identify, validate, and deserialize the content correctly.
//!
//! ## Concepts
//!
//! - **Magic Number**: A fixed identifier (`AIPL`) to distinguish files created by this application.
//! - **Version**: Tracks the version of the serialized format.
//! - **Format**: Serialization format used (`SerdeFormat`), e.g., JSON or binary.
//! - **IRKind**: Indicates the typing of intermediate representation (e.g., parsed domain, parsed problem, lifted problem).
//! - **Generated Timestamp**: Stores the UTC time when the file was created.
//!
//! The `Header` is always serialized before the payload, separated by a constant delimiter `HEADER_PAYLOAD_SEPARATOR`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::ir::header::{Header, MAGIC, HEADER_PAYLOAD_SEPARATOR};
//! use aiplan4rust::artefact::ir::kind::IRKind;
//! use aiplan4rust::serialization::serde::SerdeFormat;
//!
//! // Create a new header for a parsed domain serialized in JSON
//! let header = Header::new(SerdeFormat::Json, 1, IRKind::ParsedDomain);
//!
//! // Access metadata
//! assert_eq!(header.magic(), MAGIC);
//! assert_eq!(header.ir_kind(), IRKind::ParsedDomain);
//!
//! // Validate magic number
//! assert!(header.validate_magic());
//!
//! // Display the header
//! println!("{}", header);
//! ```

use crate::aiplan4rust::cli::io::artefact::ir::kind::IRKind;
use crate::aiplan4rust::cli::io::serialization::serde::SerdeFormat;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Fixed identifier for all serialized files produced by the application.
pub const MAGIC: &str = "AIPL";

/// Delimiter used to separate the header from the payload in a serialized file.
pub const HEADER_PAYLOAD_SEPARATOR: &str = "\n---AIPL-HEADER---\n";

/// Represents the metadata header for a serialized IR file.
///
/// The header is serialized before the file payload and contains information necessary
/// for correct deserialization and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    magic: String,
    version: u8,
    format: SerdeFormat,
    kind: IRKind,
    generated_at: String,
}

impl Header {
    /// Constructs a new `Header` with the specified serialization format, version, and IR kind.
    ///
    /// The `generated_at` timestamp is automatically set to the current UTC time in RFC3339 format.
    ///
    /// # Arguments
    ///
    /// * `format` - Serialization format used for the payload (`SerdeFormat`).
    /// * `version` - Format version number.
    /// * `kind` - Type of intermediate representation (`IRKind`).
    ///
    /// # Returns
    ///
    /// A `Header` instance ready for serialization.
    pub fn new(format: SerdeFormat, version: u8, kind: IRKind) -> Self {
        Self {
            magic: MAGIC.to_string(),
            version,
            format,
            kind,
            generated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Returns the magic number of this header.
    pub fn magic(&self) -> &str {
        &self.magic
    }

    /// Returns the version number of the serialized format.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the serialization format used.
    pub fn format(&self) -> SerdeFormat {
        self.format
    }

    /// Returns the typing of intermediate representation.
    pub fn ir_kind(&self) -> IRKind {
        self.kind
    }

    /// Returns the UTC timestamp when the file was generated.
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    /// Checks whether the magic number matches the expected `MAGIC` constant.
    ///
    /// # Returns
    ///
    /// `true` if the header's magic number is valid, `false` otherwise.
    pub fn check_magic(&self) -> bool {
        self.magic == MAGIC
    }
}

impl Display for Header {
    /// Formats the header for human-readable output.
    ///
    /// # Example
    ///
    /// ```
    /// let header = Header::new(SerdeFormat::Json, 1, IRKind::ParsedDomain);
    /// println!("{}", header);
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header {{")?;
        writeln!(f, "    magic       : {}", self.magic())?;
        writeln!(f, "    version     : {}", self.version())?;
        writeln!(f, "    format      : {}", self.format())?;
        writeln!(f, "    IR kind     : {}", self.ir_kind())?;
        writeln!(f, "    generated_at: {}", self.generated_at())?;
        write!(f, "}}")
    }
}
