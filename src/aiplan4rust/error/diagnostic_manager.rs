//! This module provides the `DiagnosticManager` struct,
//! which manages the collection and display of diagnostics
//! (errors, warnings, and informational messages) associated
//! with parsing and analysis tasks.
//!
//! It also handles associated source files so diagnostics
//! can be contextualized with original source code.

use std::collections::HashMap;
use crate::aiplan4rust::error::{Diagnostic, DiagnosticKind, DiagnosticSeverity};

/// Manages a collection of diagnostics and associated source files.
///
/// The `DiagnosticManager` is used to track, organize, and retrieve
/// diagnostics (errors, warnings, etc.) that are generated during
/// parsing, analysis, or compilation. It also stores the source files
/// corresponding to those diagnostics to provide contextual messages.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticManager {
    diagnostics: Vec<Diagnostic>,
    sources: HashMap<String, String>,
}

impl DiagnosticManager {
    /// Creates a new, empty `DiagnosticManager`.
    pub fn new() -> Self {
        DiagnosticManager {
            diagnostics: Vec::new(),
            sources: HashMap::new(),
        }
    }

    /// Adds a source file to the manager.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name or path of the file.
    /// * `source` - The contents of the file.
    pub fn add_source(&mut self, filename: String, source: String) {
        self.sources.insert(filename, source);
    }

    /// Retrieves a reference to a stored source file by filename.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the file to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<&String>` with the contents if the file exists.
    pub fn get_source(&self, filename: &str) -> Option<&String> {
        self.sources.get(filename)
    }

    /// Adds a single diagnostic to the manager.
    ///
    /// # Arguments
    ///
    /// * `diagnostic` - The diagnostic to add.
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Adds a batch of diagnostics to the manager.
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - A vector of diagnostics to add.
    pub fn add_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Returns an iterator over all diagnostics.
    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Checks if there are any diagnostics of a specific kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of diagnostic to look for.
    ///
    /// # Returns
    ///
    /// `true` if at least one diagnostic of the given kind exists.
    pub fn has_diagnotics_of_kind(&self, kind: DiagnosticKind) -> bool {
        self.diagnostics.iter().any(|e| *e.kind() == kind)
    }

    /// Checks if there are any diagnostics of a specific severity.
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity level to check for.
    ///
    /// # Returns
    ///
    /// `true` if at least one diagnostic of the given severity exists.
    pub fn has_diagnotics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.diagnostics.iter().any(|e| e.kind().severity() == severity)
    }

    /// Removes all diagnostics and sources, resetting the manager to its initial state.
    pub fn reset(&mut self) {
        self.diagnostics.clear();
        self.sources.clear();
    }

    /// Copies diagnostics and sources from another manager.
    ///
    /// # Arguments
    ///
    /// * `other` - The other `DiagnosticManager` to merge from.
    pub fn add_diagnostic_from(&mut self, other: &DiagnosticManager) {
        self.diagnostics.extend(other.diagnostics.iter().cloned());
        self.sources.extend(other.sources.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    /// Returns `true` if no diagnostics are currently stored.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Prints all diagnostics to the standard output, sorted by position.
    ///
    /// Diagnostics are ordered by line and column to provide clear,
    /// readable error reporting.
    pub fn display_all(&self) {
        let mut sorted_errors = self.diagnostics.clone();

        // Sort diagnostics by line and column
        sorted_errors.sort_by_key(|e| (e.span().begin_line(), e.span().begin_column()));

        for diagnostic in sorted_errors {
            println!("{}", diagnostic);
        }
    }
}
