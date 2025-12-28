//! Module defining file headers for serialized objects.

use std::fmt::Display;
use chrono::Utc;
use serde::{Serialize, Deserialize};
use crate::aiplan4rust::artefact::ir::kind::IRKind;
use crate::aiplan4rust::serialization::serde::SerdeFormat;

/// Fixed-size magic number for identifying files produced by the application.
pub const MAGIC: &str = "AIPL";

pub const HEADER_PAYLOAD_SEPARATOR: &str = "\n---\n";

/// Represents the header for a serialized file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    magic: String,
    version: u8,
    format: SerdeFormat,
    kind: IRKind,
    generated_at: String,
}

impl Header {
    pub fn new(format: SerdeFormat, version: u8, kind: IRKind) -> Self {
        Self {
            magic: MAGIC.to_string(),
            version,
            format,
            kind,
            generated_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn magic(&self) -> &str { &self.magic }
    pub fn version(&self) -> u8 { self.version }
    pub fn format(&self) -> SerdeFormat { self.format }
    pub fn ir_kind(&self) -> IRKind { self.kind }
    pub fn generated_at(&self) -> &str { &self.generated_at }
    pub fn validate_magic(&self) -> bool { self.magic == MAGIC }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header {{")?;
        writeln!(f, "    magic       : {}", self.magic)?;
        writeln!(f, "    version     : {}", self.version)?;
        writeln!(f, "    format      : {}", self.format)?;
        writeln!(f, "    IR kind     : {}", self.kind)?;
        writeln!(f, "    generated_at: {}", self.generated_at)?;
        write!(f, "}}")
    }
}
