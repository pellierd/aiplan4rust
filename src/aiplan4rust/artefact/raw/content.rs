//! Module representing raw planning file content.
//!
//! This module defines the `RawContent` struct, which encapsulates the
//! contents of a raw planning file (PDDL or HDDL). It stores the type of
//! the file (`RawKind`), its planning language (`Language`), and the actual
//! textual content. `RawContent` is typically used as part of the `Source`
//! abstraction in the planning pipeline.
//!
//! # Concepts
//!
//! - **RawKind**: Specifies whether the raw content is a domain file or a problem file.
//! - **Language**: Specifies the planning language of the content (e.g., PDDL, HDDL).
//! - **inner**: The raw text of the file.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::artefact::raw::RawContent;
//! use aiplan4rust::artefact::RawKind;
//! use aiplan4rust::Language;
//!
//! let raw_content = RawContent::new(RawKind::Domain, Language::PDDL, "(define (domain blocks))");
//!
//! assert_eq!(raw_content.kind(), RawKind::Domain);
//! assert_eq!(raw_content.language(), Language::PDDL);
//! assert_eq!(raw_content.inner(), "(define (domain blocks))");
//! println!("{}", raw_content);
//! ```

use std::fmt;
use crate::aiplan4rust::artefact::RawKind;
use crate::Language;

/// Represents the contents of a raw planning file (domain or problem).
///
/// `RawContent` stores:
/// - The type of the file (`RawKind`),
/// - The planning language (`Language`),
/// - The actual textual content (`inner`).
///
/// This struct is immutable and is typically used within a `Source`
/// to represent raw PDDL/HDDL files.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawContent {
    kind: RawKind,
    language: Language,
    inner: String,
}

impl RawContent {
    /// Creates a new `RawContent` instance.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the raw file (`Domain` or `Problem`).
    /// * `language` - The planning language of the content (e.g., PDDL, HDDL).
    /// * `inner` - The textual content of the raw file.
    ///
    /// # Returns
    ///
    /// A `RawContent` instance encapsulating the file metadata and content.
    pub fn new(kind: RawKind, language: Language, inner: impl Into<String>) -> Self {
        Self {
            kind,
            language,
            inner: inner.into(),
        }
    }

    /// Returns the kind of this raw content (`Domain` or `Problem`).
    pub fn kind(&self) -> RawKind {
        self.kind
    }

    /// Returns the planning language of this raw content.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Returns a reference to the textual content of this raw file.
    pub fn inner(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for RawContent {
    /// Formats the `RawContent` as a human-readable string.
    ///
    /// Displays the kind, language, and length of the content.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::artefact::raw::RawContent;
    /// # use aiplan4rust::artefact::RawKind;
    /// # use aiplan4rust::Language;
    /// let raw = RawContent::new(RawKind::Domain, Language::PDDL, "(define (domain blocks))");
    /// println!("{}", raw);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RawContent {{ kind: {}, language: {}, length: {} }}",
            self.kind,
            self.language,
            self.inner.len()
        )
    }
}
