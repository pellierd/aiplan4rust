//! Provides the [`DiagnosticManager`] struct,
//! which manages diagnostics (errors, warnings, informational messages)
//! produced during parsing, semantic analysis, and compilation phases.
//!
//! It also stores source files to associate diagnostics with concrete source
//! locations for better error reporting and user feedback.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, Severity};
use crate::aiplan4rust::interner::{Ident, Literal};

use std::collections::HashMap;
use std::fmt;
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
    sources: HashMap<Literal, String>,
}

impl DiagnosticManager {
    /// Creates a new, empty [`DiagnosticManager`].
    pub fn new() -> Self {
        DiagnosticManager {
            diagnostics: Vec::new(),
            sources: HashMap::new(),
        }
    }

    /// Registers the contents of a source file for diagnostic rendering.
    ///
    /// This method associates a given interned `Literal` (which serves as a file identifier)
    /// with the full source text of that file. This is required for computing source
    /// spans, line/column positions, and for displaying annotated diagnostics to the user.
    ///
    /// # Arguments
    ///
    /// * `literal` - An interned identifier (`Literal`) representing the source file.
    /// * `source` - The full contents of the source file as a `String`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// diagnostic_manager.add_source(file_id, source_code.to_string());
    /// ```
    ///
    /// # Notes
    ///
    /// - The `Literal` typically comes from a `StringInterner`.
    /// - The associated content can later be used for rendering spans, snippets, and context
    ///   in error messages.
    pub fn add_source(&mut self, literal: Literal, source: String) {
        self.sources.insert(literal, source);
    }

    /// Retrieves the source content associated with a given interned file identifier.
    ///
    /// This method returns the full source text that was previously registered
    /// with the corresponding `Literal` identifier (typically using `add_source`).
    /// The `Literal` is an interned value used to uniquely identify source files
    /// without storing their full string path repeatedly.
    ///
    /// # Arguments
    ///
    /// * `literal` - A `Literal`, which uniquely identifies a source file.
    ///
    /// # Returns
    ///
    /// * `Some(&String)` if the file has been registered.
    /// * `None` if no source is associated with the given identifier.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(source) = diagnostic_manager.get_source(&file_id) {
    ///     println!("Source length: {}", source.len());
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - Useful for rendering diagnostics with contextual source code.
    /// - The identifier must match one previously added with `add_source`.
    pub fn get_source(&self, literal: Literal) -> Option<&String> {
        self.sources.get(&literal)
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

    /// Remaps all `Ident` values in the diagnostics managed by this `DiagnosticManager`.
    ///
    /// This is useful when merging or linking components (like domain and problem files)
    /// that use different `Ident` instances but refer to the same logical symbols.
    ///
    /// This method applies the given mapping to each individual [`Diagnostic`] in the manager.
    ///
    /// # Arguments
    ///
    /// * `map` - A mapping of old [`Ident`]s to new [`Ident`]s.
    ///
    /// # Example
    /// ```
    /// let mut manager = DiagnosticManager::default();
    /// let mut map = HashMap::new();
    /// map.insert(old_id, new_id);
    /// manager.remap(&map);
    /// ```
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        for diagnostic in &mut self.diagnostics {
            diagnostic.remap_idents(map);
        }
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

}

/// Implements the `Display` trait for `DiagnosticManager` for debugging purposes.
///
/// This implementation provides a structured debug output that includes:
/// - The total number of diagnostics and sources.
/// - A list of diagnostics sorted by their starting span position,
///   with index, kind, provider, source identifier, and span.
/// - A preview of each registered source file (first 30 characters).
///
/// This format is intended to help developers understand the internal state
/// of the `DiagnosticManager` during debugging sessions or test assertions,
/// and is **not** designed for end-user output.
///
/// # Example Output
///
/// ```text
/// === DiagnosticManager Debug Dump ===
/// Total diagnostics: 2
/// Total sources: 1
/// --- Diagnostics ---
/// [0] kind: UnexpectedToken, provider: Parser, source: Literal(0), span: Span { start_line: 4, start_col: 12, end_line: 4, end_col: 16 }
/// [1] kind: UnusedSymbol, provider: Validator, source: Literal(1), span: Span { start_line: 10, start_col: 1, end_line: 10, end_col: 7 }
/// --- Sources ---
/// Literal(0) => "(define (domain blocks) (:predic..."
/// Literal(1) => "(define (problem test) (:init ..."
/// ```
impl fmt::Display for DiagnosticManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== DiagnosticManager Debug Dump ===")?;
        writeln!(f, "Total diagnostics: {}", self.diagnostics.len())?;
        writeln!(f, "Total sources: {}", self.sources.len())?;
        writeln!(f, "--- Diagnostics ---")?;

        // Clone and sort diagnostics by span start
        let mut diagnostics = self.diagnostics.clone();
        diagnostics.sort_by_key(|d| d.span().start());

        for (i, diag) in diagnostics.iter().enumerate() {
            writeln!(
                f,
                "[{}] kind: {}, provider: {}, source: {}, span: {}",
                i,
                diag.kind(),      // Assuming `Kind` holds the variant
                diag.provider(),
                diag.source(),
                diag.span()
            )?;
        }

        writeln!(f, "--- Sources ---")?;
        for (literal, content) in &self.sources {
            let preview: String = content.chars().take(30).collect();
            writeln!(f, "{} => \"{}...\"", literal, preview)?;
        }

        Ok(())
    }
}
