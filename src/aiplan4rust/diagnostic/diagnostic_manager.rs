//! Provides the [`DiagnosticManager`] struct,
//! which manages diagnostics (errors, warnings, informational messages)
//! produced during parsing, semantic analysis, and compilation phases.
//!
//! It also stores source files to associate diagnostics with concrete source
//! locations for better error reporting and user feedback.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, Severity};
use std::collections::HashMap;
use itertools::Itertools;

/// Manages a collection of diagnostics and their associated source files.
///
/// The [`DiagnosticManager`] serves as a centralized collector and organizer for diagnostics
/// produced during various stages of processing (e.g., parsing, semantic analysis).
///
/// It also stores source file contents to contextualize error messages with
/// relevant code snippets.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticManager {
    diagnostics: Vec<Diagnostic>,
    sources: HashMap<String, String>,
}

impl DiagnosticManager {
    /// Creates a new, empty [`DiagnosticManager`].
    pub fn new() -> Self {
        DiagnosticManager {
            diagnostics: Vec::new(),
            sources: HashMap::new(),
        }
    }

    /// Adds a source file for diagnostic context.
    ///
    /// # Arguments
    ///
    /// * `filename` - A name or path identifying the source file.
    /// * `source` - The full contents of the source file.
    pub fn add_source(&mut self, filename: String, source: String) {
        self.sources.insert(filename, source);
    }

    /// Retrieves the source content associated with a file name.
    ///
    /// # Arguments
    ///
    /// * `filename` - The identifier for the source file.
    ///
    /// # Returns
    ///
    /// * `Some(&String)` if the file exists.
    /// * `None` otherwise.
    pub fn get_source(&self, filename: &str) -> Option<&String> {
        self.sources.get(filename)
    }

    /// Adds a single diagnostic to the collection.
    ///
    /// # Arguments
    ///
    /// * `diagnostic` - A diagnostic message (error, warning, etc.).
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Adds multiple diagnostics at once.
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - A list of diagnostic messages to add.
    pub fn add_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Returns an iterator over all diagnostics sorted by position.
    ///
    /// Diagnostics are sorted by their starting character offset.
    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .sorted_by_key(|d| d.span().start())
    }

    /// Checks if at least one diagnostic of a specific kind exists.
    ///
    /// # Arguments
    ///
    /// * `kind` - The [`DiagnosticKind`] to look for.
    ///
    /// # Returns
    ///
    /// `true` if at least one matching diagnostic is found.
    pub fn has_diagnotics_of_kind(&self, kind: DiagnosticKind) -> bool {
        self.diagnostics.iter().any(|e| *e.kind() == kind)
    }

    /// Checks if any diagnostic has the given severity.
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity level to filter by.
    ///
    /// # Returns
    ///
    /// `true` if at least one diagnostic has the specified severity.
    pub fn has_diagnostics_of_severity(&self, severity: Severity) -> bool {
        self.diagnostics.iter().any(|e| e.kind().severity() == severity)
    }

    /// Returns the number of diagnostics with the specified severity.
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity level to count.
    pub fn count_diagnostics_of_severity(&self, severity: Severity) -> usize {
        self.diagnostics.iter()
            .filter(|e| e.kind().severity() == severity)
            .count()
    }

    /// Clears all diagnostics and sources, resetting the manager.
    pub fn reset(&mut self) {
        self.diagnostics.clear();
        self.sources.clear();
    }

    /// Adds diagnostics and sources from another [`DiagnosticManager`].
    ///
    /// # Arguments
    ///
    /// * `other` - Another diagnostic manager whose contents will be merged.
    pub fn add_diagnostic_from(&mut self, other: DiagnosticManager) {
        self.diagnostics.extend(other.diagnostics);
        self.sources.extend(other.sources);
    }

    /// Returns `true` if no diagnostics are stored.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Displays all diagnostics to stdout, sorted by line and column.
    ///
    /// This is useful for CLI tools or debugging purposes.
    pub fn display_all(&self) {
        let mut sorted_errors = self.diagnostics.clone();
        sorted_errors.sort_by_key(|e| (e.span().begin_line(), e.span().begin_column()));

        for diagnostic in sorted_errors {
            println!("{}", diagnostic);
        }
    }
}
