//! Artefact module: managing sources for the planning pipeline
//!
//! This module provides abstractions and utilities for handling different types of sources
//! (raw, intermediate representation, unknown text, unknown binary) used in the planning pipeline.
//! It allows reading, identifying, and accessing content in a typing-safe way while preserving
//! file provenance information.
//!
//! # Concepts
//!
//! - **Source**: Represents a single input artifact. A `Source` can be:
//!   - `Raw`: A raw textual source, e.g., PDDL or HDDL.
//!   - `IR`: An intermediate representation of a domain or problem.
//!   - `Text`: An unknown text source whose typing could not be determined.
//!   - `Binary`: An unknown binary source.
//!
//! - **ArtefactError**: Enumerates all possible errors that can occur while reading,
//!   analyzing, or accessing a source. Includes I/O errors, JSON serialization errors,
//!   missing content, or invalid file names.
//!
//! # Usage
//!
//! To read a file and obtain a `Source`:
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::{Source, ArtefactError};
//! use std::path::Path;
//!
//! let source: Source = Source::read_from_file(Path::new("domain.pddl"))?;
//! match source {
//!     Source::Raw { .. } => println!("Raw source detected"),
//!     Source::IR { .. } => println!("IR source detected"),
//!     Source::Text { .. } | Source::Binary { .. } => println!("Unknown source typing"),
//! }
//! # Ok::<(), ArtefactError>(())
//! ```
//!
//! # Accessing content
//!
//! Each `Source` variant provides safe accessors:
//!
//! - `raw_content` / `try_raw_content` for `Raw` sources.
//! - `ir_content` / `try_ir_content` for `IR` sources.
//! - `text_content` / `try_text_content` for unknown text sources.
//! - `binary_content` / `try_binary_content` for unknown binary sources.
//!
//! # Detection helpers
//!
//! `Source` provides utility methods to check its typing and contents:
//! - `is_raw`, `is_ir`, `is_text`, `is_binary`
//! - `is_raw_domain`, `is_raw_problem`, `is_raw_pddl`, `is_raw_hddl`
//! - `is_parsed_domain`, `is_parsed_problem`, `is_lifted_problem`
//!
//! These helpers make it easy to branch ops depending on the typing of the source.
//!
//! # Example: Reading and accessing content
//!
//! ```rust,ignore
//! let source = Source::read_from_file("domain.pddl")?;
//! if let Ok(raw) = source.try_raw_content() {
//!     println!("Language: {:?}", raw.language());
//!     println!("Kind: {:?}", raw.kind());
//! }
//! ```

use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::artefact::ir::content::IRContent;
use crate::aiplan4rust::artefact::ir::header::{Header, HEADER_PAYLOAD_SEPARATOR};
use crate::aiplan4rust::artefact::language::Language;
use crate::aiplan4rust::artefact::raw::content::RawContent;
use crate::aiplan4rust::artefact::raw::kind::RawKind;
use crate::aiplan4rust::artefact::IRKind;
use crate::aiplan4rust::lir::problem::problem::Problem;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::serialization::{SerdeSerializable, SerializationError};
use crate::aiplan4rust::syntax::lexer::Token;
use logos::Logos;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Represents a source artifact used in the planning pipeline.
///
/// A `Source` can be one of several types, representing different stages or forms of input:
/// - `Raw`: A textual source in PDDL or HDDL format, containing a `RawContent`.
/// - `IR`: An intermediate representation (IR) of a domain or problem, containing an `IRContent`.
/// - `Text`: An unknown text source whose structure or typing could not be determined.
/// - `Binary`: An unknown binary source whose content could not be classified.
///
/// Each variant stores the path from which the source was read, allowing for
/// traceability and file-based operations.
///
/// # Examples
///
/// ```rust,ignore
/// use std::path::PathBuf;
/// use aiplan4rust::artefact::{Source, RawContent, IRContent};
///
/// let raw_source = Source::Raw {
///     path: PathBuf::from("domain.pddl"),
///     content: RawContent::new("..."),
/// };
///
/// let ir_source = Source::IR {
///     path: PathBuf::from("domain.ir"),
///     content: IRContent::ParsedDomain(Default::debug(), "format".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub enum Source {
    /// A raw textual source, typically PDDL or HDDL.
    /// Contains the original file path and the raw content.
    Raw {
        /// Path to the raw source file.
        path: PathBuf,
        /// Raw content of the source.
        content: RawContent,
    },

    /// An intermediate representation (IR) source of a domain or problem.
    /// Contains the file path and IR content, which may include parsed domains or problems.
    IR {
        /// Path to the IR source file.
        path: PathBuf,
        /// Content of the IR source.
        content: IRContent,
    },

    /// An unknown text source, where the typing or structure could not be determined.
    /// Stores the file path and raw string content.
    Text {
        /// Path to the text source file.
        path: PathBuf,
        /// Raw text content.
        content: String,
    },

    /// An unknown binary source, where the typing could not be determined.
    /// Stores the file path and binary content.
    Binary {
        /// Path to the binary source file.
        path: PathBuf,
        /// Raw binary content.
        content: Vec<u8>,
    },
}

impl Source {
    /// Constructs a new `Source::Raw` variant.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path associated with the raw source.
    /// * `content` - The raw content of the source (PDDL or HDDL).
    ///
    /// # Returns
    ///
    /// A `Source` instance representing a raw source.
    fn new_raw(path: impl Into<PathBuf>, content: RawContent) -> Self {
        Source::Raw {
            path: path.into(),
            content,
        }
    }

    /// Constructs a new `Source::IR` (Intermediate Representation) variant.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path associated with the IR source.
    /// * `content` - The IR content of the source.
    ///
    /// # Returns
    ///
    /// A `Source` instance representing an IR source.
    fn new_ir(path: impl Into<PathBuf>, content: IRContent) -> Self {
        Source::IR {
            path: path.into(),
            content,
        }
    }

    /// Constructs a new `Source::Text` variant for unknown text content.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path associated with the text source.
    /// * `content` - The unknown text content as a string.
    ///
    /// # Returns
    ///
    /// A `Source` instance representing an unknown text source.
    #[allow(dead_code)]
    fn new_text(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Source::Text {
            path: path.into(),
            content: content.into(),
        }
    }

    /// Constructs a new `Source::Binary` variant for unknown binary content.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path associated with the binary source.
    /// * `content` - The unknown binary content as a byte vector.
    ///
    /// # Returns
    ///
    /// A `Source` instance representing an unknown binary source.
    #[allow(dead_code)]
    fn new_binary(path: impl Into<PathBuf>, content: Vec<u8>) -> Self {
        Source::Binary {
            path: path.into(),
            content,
        }
    }

    /// Attempts to create a `Source` from any typing that implements `AsRef<Path>`.
    ///
    /// This is a convenience wrapper around `TryFrom<&Path>` which allows passing
    /// `Path`, `PathBuf`, or `&str`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::path::Path;
    /// use aiplan4rust::artefact::{Source, ArtefactError};
    ///
    /// let source = Source::try_from_path("domain.pddl")?;
    /// # Ok::<(), ArtefactError>(())
    /// ```
    pub fn try_from_path<P>(path: P) -> Result<Self, ArtefactError>
    where
        P: AsRef<Path>,
    {
        Source::try_from(path.as_ref())
    }
}

impl TryFrom<&Path> for Source {
    type Error = ArtefactError;

    /// Attempts to create a `Source` by reading the file at the given path.
    ///
    /// This function reads the file content and automatically determines
    /// the appropriate `Source` variant:
    /// - `Raw` for PDDL or HDDL sources
    /// - `IR` for intermediate representation
    /// - `Text` or `Binary` if detection fails
    ///
    /// # Arguments
    ///
    /// * `path` - Reference to the file path to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Source)` containing the correctly detected variant.
    /// * `Err(ArtefactError)` if reading the file fails or content analysis fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::convert::TryFrom;
    /// use std::path::Path;
    /// use aiplan4rust::artefact::{Source, ArtefactError};
    ///
    /// let source = Source::try_from(Path::new("domain.pddl"))?;
    /// match source {
    ///     Source::Raw { .. } => println!("Raw source detected"),
    ///     Source::IR { .. } => println!("IR source detected"),
    ///     Source::Text { .. } | Source::Binary { .. } => println!("Unknown source typing"),
    /// }
    /// # Ok::<(), ArtefactError>(())
    /// ```
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let path_buf = path.to_path_buf();

        // Read the file bytes using the internal helper
        let bytes = Self::read_file(&path_buf)?;

        // Detect the correct Source variant based on the bytes
        Self::read_from_bytes(path_buf, bytes)
    }
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

impl Source {
    /// Reads an `Source` from a byte vector, attempting IR deserialization first, then falling back
    /// to raw or unknown content.
    ///
    /// This function performs the following steps:
    /// 1. Attempts to parse a JSON header and payload using [`Self::try_to_read_ir`].
    ///    - If a valid header is found, the payload is deserialized according to the IRKind and format.
    /// 2. If no header is found, attempts to interpret the bytes as UTF-8:
    ///    - If the text corresponds to a recognized raw kind, returns `Input::Raw`.
    ///    - Otherwise, returns `Input::TextUnknown`.
    /// 3. If UTF-8 conversion fails, returns `Input::BinaryUnknown`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the source file, used for tracking and errors.
    /// * `bytes` - Raw bytes read from the source file.
    ///
    /// # Returns
    ///
    /// Returns a fully constructed [`Source`] variant based on the content and detected format.
    ///
    /// # Errors
    ///
    /// Propagates any deserialization errors from:
    /// - UTF-8 conversion (`std::str::Utf8Error`)
    /// - IR payload deserialization (`SerializationError`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # use aiplan4rust::ir::{Input, IRKind, SemanticContext, LiftedProblem};
    /// let bytes: Vec<u8> = std::fs::read("example.ir").unwrap();
    /// let input = Input::read_from_bytes(PathBuf::from("example.ir"), bytes).unwrap();
    /// ```
    fn read_from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self, ArtefactError> {
        // Attempt to read IR header and payload
        match Self::try_to_read_ir(&bytes)? {
            Some((header, payload)) => match header.ir_kind() {
                IRKind::ParsedDomain => {
                    let sc = SemanticContext::deserialize_from_bytes(payload, header.format())?;
                    let content = IRContent::ParsedDomain(sc, header.format());
                    Ok(Source::new_ir(path, content))
                }
                IRKind::ParsedProblem => {
                    let sc = SemanticContext::deserialize_from_bytes(payload, header.format())?;
                    let content = IRContent::ParsedProblem(sc, header.format());
                    Ok(Source::new_ir(path, content))
                }
                IRKind::LiftedProblem => {
                    let pb = LiftedProblem::deserialize_from_bytes(payload, header.format())?;
                    let content = IRContent::LiftedProblem(pb, header.format());
                    Ok(Source::new_ir(path, content))
                }
                IRKind::GroundedProblem => {
                    let pb = Problem::deserialize_from_bytes(payload, header.format())?;
                    let content = IRContent::GroundedProblem(pb, header.format());
                    Ok(Source::new_ir(path, content))
                }
            },
            None => {
                // No IR header found → fallback to UTF-8
                match String::from_utf8(bytes) {
                    Ok(content) => {
                        if let Some(kind) = infer_raw_kind(&content) {
                            let language = detect_raw_language(&content);
                            let content = RawContent::new(kind, language, content);
                            return Ok(Source::new_raw(path, content));
                        }

                        Ok(Source::Text { path, content })
                    }
                    Err(e) => {
                        // Binary unknown
                        Ok(Source::Binary {
                            path,
                            content: e.into_bytes(),
                        })
                    }
                }
            }
        }
    }

    /// Reads the entire content of a file into a `Vec<u8>`.
    ///
    /// This function attempts to open the file at the given path and read all its bytes
    /// into a vector. It returns an error if the file cannot be opened or if reading
    /// fails for any reason (e.g., I/O error, permissions issue).
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `Path` representing the file to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing all bytes of the file if successful.
    /// * `Err(IOError)` if opening or reading the file fails.
    ///
    /// # Errors
    ///
    /// This function can return the following errors wrapped in `IOError`:
    /// - `IOError::open_failed(path, e)` if the file cannot be opened.
    /// - `IOError::read_failed(path, e)` if reading the file fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use aiplan4rust::io::error::IOError;
    /// use aiplan4rust::source::Input;
    ///
    /// let path = Path::new("example.pddl");
    /// match Input::read_file(path) {
    ///     Ok(bytes) => println!("Read {} bytes from the file.", bytes.len()),
    ///     Err(err) => eprintln!("Failed to read file: {:?}", err),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - This function reads the entire file into memory. For very large files, this may
    ///   cause high memory usage. Consider using buffered reading if needed.
    /// - Unlike `read_to_string`, this does not attempt to interpret the bytes as UTF-8.
    fn read_file(path: &Path) -> Result<Vec<u8>, ArtefactError> {
        let mut file = File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    /// Attempts to read an intermediate representation (IR) from a byte slice by parsing a header and payload.
    ///
    /// This function looks for a `HEADER_PAYLOAD_SEPARATOR` in the byte slice to split the header from the payload.
    /// The header is expected to be in JSON format and contains metadata including a magic number and format information.
    ///
    /// # Behavior
    /// - If the separator is **not found**, this function returns `Ok(None)`, indicating that no IR header was present.
    /// - If the separator is found, the header is parsed as UTF-8 and then deserialized as a `Header`.
    /// - If the magic number in the header is invalid, a `SerializationError::InvalidMagic` is returned.
    /// - On success, returns `Ok(Some((header, payload)))`, where `payload` is a slice of the original bytes after the separator.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A byte slice containing the potentially serialized IR data (header + payload).
    ///
    /// # Returns
    ///
    /// - `Ok(Some((Header, &[u8])))` if a valid header is found and parsed successfully.
    /// - `Ok(None)` if no header separator is found in the input bytes.
    /// - `Err(SerializationError)` if there is a UTF-8 conversion error, JSON deserialization error, or the header's magic number is invalid.
    ///
    /// # Errors
    ///
    /// This function may return the following errors wrapped in `SerializationError`:
    /// - [`SerializationError::Utf8`] if the header bytes cannot be interpreted as UTF-8.
    /// - [`SerializationError::SerdeJson`] if the header JSON is malformed.
    /// - [`SerializationError::InvalidMagic`] if the header's magic number is incorrect.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use aiplan4rust::serialization::{SerializationError, try_to_read_ir};
    /// # use aiplan4rust::io::header::HEADER_PAYLOAD_SEPARATOR;
    /// # use aiplan4rust::io::header::Header;
    /// let mut bytes = b"{\"magic\":\"AIPL\",\"version\":1}".to_vec();
    /// bytes.extend_from_slice(HEADER_PAYLOAD_SEPARATOR.as_bytes());
    /// bytes.extend_from_slice(b"{\"some\":\"payload\"}");
    ///
    /// let result = try_to_read_ir(&bytes).unwrap();
    /// assert!(result.is_some());
    /// let (header, payload) = result.unwrap();
    /// assert_eq!(payload, b"{\"some\":\"payload\"}");
    /// ```
    fn try_to_read_ir(bytes: &[u8]) -> Result<Option<(Header, &[u8])>, SerializationError> {
        let sep_index = match bytes
            .windows(HEADER_PAYLOAD_SEPARATOR.len())
            .position(|window| window == HEADER_PAYLOAD_SEPARATOR.as_bytes())
        {
            Some(i) => i,
            None => return Ok(None),
        };

        let header_bytes = &bytes[..sep_index];
        let payload = &bytes[sep_index + HEADER_PAYLOAD_SEPARATOR.len()..];

        let header_str = match std::str::from_utf8(header_bytes) {
            Ok(s) => s,
            Err(_) => return Ok(None), // invalid UTF-8 → treat as not IR
        };

        let header: Header = match serde_json::from_str(header_str) {
            Ok(h) => h,
            Err(_) => return Ok(None), // invalid JSON → treat as not IR
        };

        if !header.check_magic() {
            return Ok(None); // invalid magic → treat as not IR
        }

        Ok(Some((header, payload)))
    }
}

// -------------------------------------------------------------------------
// Accessors for Source paths and content
// -------------------------------------------------------------------------
impl Source {
    /// Returns the file path associated with this `Source`.
    ///
    /// Works for all variants (`Raw`, `IR`, `Text`, `Binary`).
    ///
    /// # Example
    /// ```rust,ignore
    /// let path = source.path();
    /// println!("Source path: {}", path.display());
    /// ```
    pub fn path(&self) -> &Path {
        match self {
            Source::Raw { path, .. } => path,
            Source::IR { path, .. } => path,
            Source::Text { path, .. } => path,
            Source::Binary { path, .. } => path,
        }
    }

    // ---------------------------------------------------------------------
    // RawContent accessors
    // ---------------------------------------------------------------------

    /// Returns a reference to the `RawContent` if this `Source` represents a raw artefact
    /// (e.g., a PDDL or HDDL file).
    ///
    /// This accessor is non-failing and returns `None` if the `Source` is not of the `Raw` variant.
    ///
    /// # Returns
    /// - `Some(&RawContent)` if the `Source` is raw.
    /// - `None` otherwise.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(raw) = source.raw_content() {
    ///     println!("Raw source kind: {:?}", raw.kind());
    /// }
    /// ```
    pub fn raw_content(&self) -> Option<&RawContent> {
        match self {
            Source::Raw { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns a reference to the `RawContent` if this `Source` represents a raw artefact,
    /// or an `ArtefactError::MissingRawContent` if it does not.
    ///
    /// This method is useful when the caller expects the `Source` to be raw and wants
    /// to fail explicitly otherwise.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingRawContent` if the `Source` is not of the `Raw` variant.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// match source.try_raw_content() {
    ///     Ok(raw) => println!("Raw source kind: {:?}", raw.kind()),
    ///     Err(e) => eprintln!("Source is not raw: {}", e),
    /// }
    /// ```
    pub fn try_raw_content(&self) -> Result<&RawContent, ArtefactError> {
        match self {
            Source::Raw { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_raw_content()),
        }
    }

    // ---------------------------------------------------------------------
    // IRContent accessors
    // ---------------------------------------------------------------------

    /// Returns a reference to the `IRContent` if this `Source` represents an intermediate
    /// representation (IR) artefact.
    ///
    /// This accessor is non-failing and returns `None` if the `Source` is not of the `IR` variant.
    ///
    /// # Returns
    /// - `Some(&IRContent)` if the `Source` is IR.
    /// - `None` otherwise.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(ir) = source.ir_content() {
    ///     println!("IR content typing: {:?}", ir.kind());
    /// }
    /// ```
    pub fn ir_content(&self) -> Option<&IRContent> {
        match self {
            Source::IR { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns a reference to the `IRContent` if this `Source` represents an IR artefact,
    /// or an `ArtefactError::MissingIRContent` if it does not.
    ///
    /// This method is useful when the caller expects the `Source` to be IR and wants
    /// to fail explicitly otherwise.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingIRContent` if the `Source` is not of the `IR` variant.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// match source.try_ir_content() {
    ///     Ok(ir) => println!("IR content typing: {:?}", ir.kind()),
    ///     Err(e) => eprintln!("Source is not IR: {}", e),
    /// }
    /// ```
    pub fn try_ir_content(&self) -> Result<&IRContent, ArtefactError> {
        match self {
            Source::IR { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_ir_content()),
        }
    }

    /// Returns a reference to the parsed semantic context if this `Source` is a parsed
    /// IR domain or problem.
    ///
    /// This is useful for extracting the `SemanticContext` of parsed IR sources
    /// without consuming the `Source`.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingIRContent` if the `Source` is not IR
    /// or does not contain a parsed domain or problem.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Ok(parsed) = source.try_parsed_content() {
    ///     println!("Parsed domain or problem context: {:?}", parsed);
    /// }
    /// ```
    pub fn try_parsed_content(&self) -> Result<&SemanticContext, ArtefactError> {
        match self {
            Source::IR { content, .. } => match content {
                IRContent::ParsedDomain(domain, _) => Ok(domain),
                IRContent::ParsedProblem(problem, _) => Ok(problem),
                _ => Err(ArtefactError::missing_ir_content()),
            },
            _ => Err(ArtefactError::missing_ir_content()),
        }
    }

    /// Returns a cloned parsed semantic context without consuming the `Source`.
    ///
    /// This method allows obtaining an owned `SemanticContext` from a parsed IR domain
    /// or problem, which can be used independently of the original `Source`.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingIRContent` if the `Source` is not IR
    /// or does not contain a parsed domain or problem.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// let owned_context = source.parsed_content_owned()?;
    /// ```
    pub fn parsed_content_owned(&self) -> Result<SemanticContext, ArtefactError> {
        match self {
            Source::IR { content, .. } => match content {
                IRContent::ParsedDomain(domain, _) => Ok(domain.clone()),
                IRContent::ParsedProblem(problem, _) => Ok(problem.clone()),
                _ => Err(ArtefactError::MissingIRContent),
            },
            _ => Err(ArtefactError::MissingIRContent),
        }
    }

    // ---------------------------------------------------------------------
    // Binary content accessors
    // ---------------------------------------------------------------------

    /// Returns a reference to the unknown binary content if this `Source` represents
    /// a binary artefact.
    ///
    /// This accessor is non-failing and returns `None` if the `Source` is not of the
    /// `Binary` variant.
    ///
    /// # Returns
    /// - `Some(&[u8])` if the `Source` is binary.
    /// - `None` otherwise.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(bytes) = source.binary_content() {
    ///     println!("Binary content length: {}", bytes.len());
    /// }
    /// ```
    pub fn binary_content(&self) -> Option<&[u8]> {
        match self {
            Source::Binary { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns a reference to the unknown binary content if this `Source` is binary,
    /// otherwise returns a `MissingBinaryContent` error.
    ///
    /// This method is useful when the caller expects the `Source` to be binary
    /// and wants to fail explicitly if it is not.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingBinaryContent` if the `Source` is not of the
    /// `Binary` variant.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// match source.try_binary_content() {
    ///     Ok(bytes) => println!("Binary content length: {}", bytes.len()),
    ///     Err(e) => eprintln!("Source is not binary: {}", e),
    /// }
    /// ```
    pub fn try_binary_content(&self) -> Result<&[u8], ArtefactError> {
        match self {
            Source::Binary { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_binary_content()),
        }
    }

    // ---------------------------------------------------------------------
    // Text content accessors
    // ---------------------------------------------------------------------

    /// Returns a reference to the unknown text content if this `Source` represents
    /// a text artefact.
    ///
    /// This accessor is non-failing and returns `None` if the `Source` is not of the
    /// `Text` variant.
    ///
    /// # Returns
    /// - `Some(&String)` if the `Source` is text.
    /// - `None` otherwise.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(text) = source.text_content() {
    ///     println!("Text content length: {}", text.len());
    /// }
    /// ```
    pub fn text_content(&self) -> Option<&String> {
        match self {
            Source::Text { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Returns a reference to the unknown text content if this `Source` is text,
    /// otherwise returns a `MissingTextContent` error.
    ///
    /// This method is useful when the caller expects the `Source` to be text
    /// and wants to fail explicitly if it is not.
    ///
    /// # Errors
    /// Returns `ArtefactError::MissingTextContent` if the `Source` is not of the
    /// `Text` variant.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// match source.try_text_content() {
    ///     Ok(text) => println!("Text content length: {}", text.len()),
    ///     Err(e) => eprintln!("Source is not text: {}", e),
    /// }
    /// ```
    pub fn try_text_content(&self) -> Result<&String, ArtefactError> {
        match self {
            Source::Text { content, .. } => Ok(content),
            _ => Err(ArtefactError::missing_text_content()),
        }
    }

    // ---------------------------------------------------------------------
    // Type predicates
    // ---------------------------------------------------------------------

    /// Returns `true` if this `Source` is a raw source (PDDL or HDDL).
    pub fn is_raw(&self) -> bool {
        matches!(self, Source::Raw { .. })
    }

    /// Returns `true` if this `Source` is a raw domain.
    pub fn is_raw_domain(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.kind() == RawKind::Domain)
    }

    /// Returns `true` if this `Source` is a raw problem.
    pub fn is_raw_problem(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.kind() == RawKind::Problem)
    }

    /// Returns `true` if this `Source` is a raw PDDL source.
    pub fn is_raw_pddl(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.language() == Language::PDDL)
    }

    /// Returns `true` if this `Source` is a raw HDDL source.
    pub fn is_raw_hddl(&self) -> bool {
        matches!(self, Source::Raw { content, .. } if content.language() == Language::HDDL)
    }

    /// Returns `true` if this `Source` is an intermediate representation (IR).
    pub fn is_ir(&self) -> bool {
        matches!(self, Source::IR { .. })
    }

    /// Returns `true` if this `Source` is a parsed IR domain.
    pub fn is_parsed_domain(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::ParsedDomain))
    }

    /// Returns `true` if this `Source` is a parsed IR problem.
    pub fn is_parsed_problem(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::ParsedProblem))
    }

    /// Returns `true` if this `Source` is a lifted IR problem.
    pub fn is_lifted_problem(&self) -> bool {
        matches!(self.ir_kind(), Some(IRKind::LiftedProblem))
    }

    /// Returns `true` if this `Source` is an unknown text source.
    pub fn is_text(&self) -> bool {
        matches!(self, Source::Text { .. })
    }

    /// Returns `true` if this `Source` is an unknown binary source.
    pub fn is_binary(&self) -> bool {
        matches!(self, Source::Binary { .. })
    }

    // ---------------------------------------------------------------------
    // Kind helpers
    // ---------------------------------------------------------------------

    /// Returns the specific kind of this raw source if it is raw.
    ///
    /// # Returns
    /// - `Some(RawKind)` if the `Source` is raw (PDDL or HDDL).
    /// - `None` if the `Source` is not raw.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(kind) = source.raw_kind() {
    ///     println!("Raw source kind: {:?}", kind);
    /// }
    /// ```
    pub fn raw_kind(&self) -> Option<RawKind> {
        match self {
            Source::Raw { content, .. } => Some(content.kind()),
            _ => None,
        }
    }

    /// Returns the specific kind of this IR source if it is an intermediate representation (IR).
    ///
    /// # Returns
    /// - `Some(IRKind)` if the `Source` is IR.
    /// - `None` if the `Source` is not IR.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let source: Source = ...;
    /// if let Some(ir_kind) = source.ir_kind() {
    ///     println!("IR source kind: {:?}", ir_kind);
    /// }
    /// ```
    pub fn ir_kind(&self) -> Option<IRKind> {
        match self {
            Source::IR { content, .. } => Some(content.kind()),
            _ => None,
        }
    }

    // ---------------------------------------------------------------------
    // Domain / Problem helpers
    // ---------------------------------------------------------------------

    /// Returns `true` if this `Source` represents a planning domain.
    ///
    /// A source is considered a domain if:
    /// - It is a raw source with `RawKind::Domain`, or
    /// - It is an IR source with `IRKind::ParsedDomain`.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let domain_source: Source = ...;
    /// if domain_source.is_domain() {
    ///     println!("This source is a domain.");
    /// }
    /// ```
    pub fn is_domain(&self) -> bool {
        matches!(self.raw_kind(), Some(RawKind::Domain))
            || matches!(self.ir_kind(), Some(IRKind::ParsedDomain))
    }

    /// Returns `true` if this `Source` represents a planning problem.
    ///
    /// A source is considered a problem if:
    /// - It is a raw source with `RawKind::Problem`, or
    /// - It is an IR source with `IRKind::ParsedProblem` or `IRKind::LiftedProblem`.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let problem_source: Source = ...;
    /// if problem_source.is_problem() {
    ///     println!("This source is a problem.");
    /// }
    /// ```
    pub fn is_problem(&self) -> bool {
        matches!(self.raw_kind(), Some(RawKind::Problem))
            || matches!(self.ir_kind(), Some(IRKind::ParsedProblem))
            || matches!(self.ir_kind(), Some(IRKind::LiftedProblem))
    }
}

/// Infers the typing of a raw source based on its textual content.
///
/// This function attempts to determine whether the raw source represents
/// a PDDL/HDDL domain or problem by scanning the initial tokens for the
/// typical `(define (domain ...)` or `(define (problem ...)` patterns.
///
/// # Arguments
///
/// * `source` - A string slice containing the raw source text.
///
/// # Returns
///
/// * `Some(RawKind::Domain)` if the source appears to define a domain.
/// * `Some(RawKind::Problem)` if the source appears to define a problem.
/// * `None` if the typing cannot be inferred.
///
/// # Notes
///
/// * The function uses lexical analysis to detect the structure and ignores
///   errors or unexpected tokens during scanning.
/// * Only the first matching `(define ...)` construct is considered.
fn infer_raw_kind(source: &str) -> Option<RawKind> {
    let mut lexer = Token::lexer(source);
    while let Some(token_res) = lexer.next() {
        let token = match token_res {
            Ok(tok) => tok,
            Err(_) => continue,
        };

        // Detect the form (define (domain ...) or (define (problem ...))
        if token == Token::LParen {
            if let (Some(Ok(Token::Define)), Some(Ok(Token::LParen)), Some(Ok(next))) =
                (lexer.next(), lexer.next(), lexer.next())
            {
                match next {
                    Token::Domain => return Some(RawKind::Domain),
                    Token::Problem => return Some(RawKind::Problem),
                    _ => {}
                }
            }
        }
    }
    None
}

/// Detects the language of a raw source text, distinguishing between PDDL and HDDL.
///
/// This function analyzes the source text and attempts to identify whether it is
/// written in PDDL or HDDL. Detection is based on the presence of HDDL-specific
/// tokens such as `Task`, `Method`, `Hierarchy`, etc. If none of these tokens
/// are found, the function assumes the language is PDDL.
///
/// # Arguments
///
/// * `source` - A string slice representing the raw source text to analyze.
///
/// # Returns
///
/// * `Language::HDDL` if an HDDL-specific token is found.
/// * `Language::PDDL` otherwise.
///
/// # Notes
///
/// * Detection is based solely on token presence.
/// * Invalid or unrecognized tokens are ignored.
fn detect_raw_language(source: &str) -> Language {
    let mut lexer = Token::lexer(source);

    while let Some(token_res) = lexer.next() {
        let token = match token_res {
            Ok(tok) => tok,
            Err(_) => continue,
        };

        // HDDL-specific tokens
        match token {
            Token::Task
            | Token::Method
            | Token::MethodPreconditions
            | Token::Hierarchy
            | Token::Htn => return Language::HDDL,
            _ => {}
        }
    }

    Language::PDDL
}
