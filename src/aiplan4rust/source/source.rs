//! Module `source`
//!
//! This module provides the [`Source`] struct and associated functionality
//! to represent, read, and classify sources for the planner. Sources can be:
//! raw PDDL/HDDL text, serialized data, or unknown format.
//!
//! # Overview
//!
//! - [`Source`]: Main struct representing a source file or input content.
//!   Stores the file path, the content as a `String`, and metadata describing
//!   the type of source (`SourceInfo`).
//!
//! - [`SourceInfo`]: Enum representing the kind of source:
//!     - `Raw`: PDDL/HDDL text with a detected role and language.
//!     - `Serialized`: Pre-processed source with a `Header`.
//!     - `Unknown`: Format could not be determined.
//!
//! - [`RawInfo`]: Metadata for raw sources (role and language).
//! - [`SerializedInfo`]: Metadata for serialized sources (header).
//! - [`SourceError`]: Errors that can occur when reading or interpreting a source.
//!
//! # Usage
//!
//! ```rust
//! use std::path::PathBuf;
//! use aiplan4rust::source::{Source, SourceInfo};
//!
//! // Create a Source from a file path
//! let path = PathBuf::from("example.pddl");
//! let source = Source::from_path(&path).expect("Failed to read source file");
//!
//! // Inspect path and content
//! println!("Source path: {}", source.path().display());
//! println!("Source content length: {}", source.content().len());
//!
//! // Try accessing raw information
//! match source.try_raw_info() {
//!     Ok(raw_info) => println!("Detected role: {:?}, language: {:?}", raw_info.role(), raw_info.language()),
//!     Err(err) => eprintln!("Source is not raw: {:?}", err),
//! }
//! ```
//!
//! # Notes
//!
//! - `Source` always stores a `PathBuf` for the source, even if it originates
//!   from a string or stdin; a synthetic path can be used in that case.
//! - `Source` reads the entire content into memory once at construction.
//! - `SourceInfo` allows quickly determining whether the content is raw,
//!   serialized, or unknown without additional parsing.
//! - Reading large files may consume significant memory.
//! - `try_raw_info` and `try_serialized` provide convenient ways to access
//!   metadata while returning appropriate `SourceError`s when the type is
//!   incompatible.

use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::aiplan4rust::serialization::header::Header;
use crate::aiplan4rust::source::SourceError;
use crate::aiplan4rust::source::info::{RawInfo, SerializedInfo, SourceInfo};

/// Represents a source file or input content for the planner, which can be
/// either raw PDDL/HDDL code, serialized data, or an unknown format.
///
/// The `Source` struct encapsulates:
/// - `path`: the file path associated with the source content.
/// - `content`: the full content of the source (read once).
/// - `info`: metadata about the source, describing whether it is raw,
///   serialized, or unknown.
///
/// # Variants of source content
/// - **Raw**: PDDL or HDDL text that can be parsed into an AST.
/// - **Serialized**: Pre-processed/serialized representation of a source.
/// - **Unknown**: Content whose format could not be determined.
///
/// # Notes
/// - The `path` is always a `PathBuf` representing the source location.
///   For sources from strings or stdin, a synthetic path can be provided.
/// - The `content` is stored as a `String` and is read once during construction.
/// - The `info` field provides quick access to the type of the source and
///   related metadata (role, language, or header).
///
/// # Example
/// ```rust
/// use std::path::PathBuf;
/// use aiplan4rust::source::{Source, SourceInfo};
///
/// let path = PathBuf::from("example.pddl");
/// let content = "(define (problem example) ... )".to_string();
/// let info = SourceInfo::Unknown;
///
/// let source = Source::new(path, content, info);
/// println!("Source path: {}", source.path().display());
/// ```
#[derive(Debug, Clone)]
pub struct Source {
    path: PathBuf,
    content: String,
    info: SourceInfo,
}
impl Source {
    // Constructs a new `Source` from owned values.
    ///
    /// This method consumes the provided `path`, `content`, and `SourceInfo`
    /// and returns a fully initialized `Source` instance.
    ///
    /// # Arguments
    /// - `path`: The file path associated with this source.
    /// - `content`: The full content of the source file or input string.
    /// - `info`: The type of the source, e.g., raw PDDL/HDDL, serialized, or unknown.
    ///
    /// # Returns
    /// - A `Source` instance containing the provided path, content, and info.
    ///
    /// # Example
    /// ```rust
    /// use std::path::PathBuf;
    /// use aiplan4rust::source::{Source, SourceInfo};
    ///
    /// let path = PathBuf::from("example.pddl");
    /// let content = "(define (problem example) ... )".to_string();
    /// let info = SourceInfo::Unknown;
    ///
    /// let source = Source::new(path, content, info);
    /// ```
    pub fn new(path: PathBuf, content: String, info: SourceInfo) -> Self {
        Self {
            path,
            content,
            info,
        }
    }

    /// Returns the path of the source as a `String`.
    ///
    /// This converts the internal `PathBuf` to a string, using lossy conversion
    /// if necessary (invalid UTF-8 sequences will be replaced).
    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// Creates a `Source` from a file path.
    ///
    /// Reads the content of the file at the given path and automatically
    /// detects whether it is a raw PDDL/HDDL source, a serialized source,
    /// or unknown.
    ///
    /// # Arguments
    /// - `path`: Any type that can be converted to a `Path` reference.
    ///
    /// # Returns
    /// - `Ok(Source)`: If the file could be read and parsed successfully.
    /// - `Err(SourceError)`: If reading the file fails.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        let path_buf = path.as_ref().to_path_buf();
        let content = Self::read_file(&path_buf)?;
        Ok(Self::from_content_with_path(&content, path_buf))
    }

    /// Creates a `Source` from a string path.
    ///
    /// This is a convenience wrapper around [`Source::from_path`].
    ///
    /// # Arguments
    /// - `path_str`: A string slice representing the path to the source file.
    ///
    /// # Returns
    /// - `Ok(Source)`: If the file could be read and parsed successfully.
    /// - `Err(SourceError)`: If reading the file fails.
    pub fn from_path_str(path_str: &str) -> Result<Self, SourceError> {
        Self::from_path(path_str)
    }

    /// Creates a `Source` from file content and its path.
    ///
    /// Detects whether the content represents a raw source (PDDL/HDDL),
    /// a serialized source, or is unknown.
    ///
    /// # Arguments
    /// - `content`: The source content, can be any type convertible into `String`.
    /// - `path`: The file path associated with this source.
    ///
    /// # Returns
    /// - `Source` instance with the appropriate `SourceInfo` set.
    fn from_content_with_path(content: impl Into<String>, path: PathBuf) -> Self {
        let content = content.into();
        let info = if let Some(header) = Header::read_header_from_source_content(&content) {
            SourceInfo::Serialized(SerializedInfo::new(header))
        } else if let Some(raw_info) = RawInfo::detect_raw_info(&content) {
            SourceInfo::Raw(raw_info)
        } else {
            SourceInfo::Unknown
        };

        Self::new(path, content, info)
    }

    /// Returns a reference to the internal path of the source.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the source content string.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns a reference to the `SourceInfo` enum describing the source type.
    pub fn info(&self) -> &SourceInfo {
        &self.info
    }

    /// Returns a reference to `RawInfo` if the source is raw.
    ///
    /// # Errors
    /// - `SourceError::unexpected_serialized_source` if the source is serialized.
    /// - `SourceError::unknown_source` if the source type is unknown.
    pub fn try_raw_info(&self) -> Result<&RawInfo, SourceError> {
        match &self.info {
            SourceInfo::Raw(raw) => Ok(raw),
            SourceInfo::Serialized(_) => Err(SourceError::unexpected_serialized_source()),
            SourceInfo::Unknown => Err(SourceError::unknown_source()),
        }
    }

    /// Returns a reference to `SerializedInfo` if the source is serialized.
    ///
    /// # Errors
    /// - `SourceError::unexpected_raw_source` if the source is raw.
    /// - `SourceError::unknown_source` if the source type is unknown.
    pub fn try_serialized(&self) -> Result<&SerializedInfo, SourceError> {
        match &self.info {
            SourceInfo::Serialized(serialized) => Ok(serialized),
            SourceInfo::Raw(_) => Err(SourceError::unexpected_raw_source()),
            SourceInfo::Unknown => Err(SourceError::unknown_source()),
        }
    }

    /// Reads the content of a source file into a `String`.
    ///
    /// This function attempts to open the file at the given path and read its contents into a
    /// `String`. If the file cannot be opened or read, it returns a `SourceError` describing the problem.
    ///
    /// # Arguments
    /// - `path`: A reference to the `Path` representing the path of the source file to read.
    ///
    /// # Returns
    /// - `Ok(String)`: The file content as a `String` if reading was successful. Returns an empty
    ///   string if the file is empty.
    /// - `Err(SourceError)`: An error occurred while opening or reading the file.
    ///
    /// # Errors
    /// - `SourceError::open_failed`: If the file cannot be opened (e.g., it doesn't exist or there
    ///   are permission issues).
    /// - `SourceError::read_failed`: If the file cannot be read (e.g., I/O or encoding issues).
    ///
    /// # Special Cases
    /// - **Empty files**: Returns an empty `String`. No error is raised.
    /// - **Large files**: Reads the entire file into memory. For very large files, consider
    ///   streaming in chunks to avoid high memory usage.
    ///
    /// # Example
    /// ```rust
    /// use std::path::PathBuf;
    /// use aiplan4rust::source::Source;
    /// use aiplan4rust::source::SourceError;
    ///
    /// let path = PathBuf::from("path/to/source/file.pddl");
    /// match Source::read_file(&path) {
    ///     Ok(content) => println!("File content loaded, {} bytes", content.len()),
    ///     Err(err) => eprintln!("Failed to read file: {:?}", err),
    /// }
    /// ```
    fn read_file(path: &Path) -> Result<String, SourceError> {
        let mut content = String::new();

        let mut file = File::open(path)
            .map_err(|e| SourceError::open_failed(path.to_path_buf(), e))?;

        file.read_to_string(&mut content)
            .map_err(|e| SourceError::read_failed(path.to_path_buf(), e))?;

        Ok(content)
    }

}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Source {{")?;
        writeln!(f, "    path   : {}", self.path_str())?;
        match &self.info {
            SourceInfo::Raw(raw) => {
                writeln!(f, "    role   : {}", raw.role())?;
                writeln!(f, "    language: {}", raw.language())?;
            }
            SourceInfo::Serialized(serialized) => {
                writeln!(f, "    header : {}", serialized.header())?;
            }
            SourceInfo::Unknown => {
                writeln!(f, "    info   : Unknown")?;
            }
        }
        writeln!(f, "}}")
    }
}
