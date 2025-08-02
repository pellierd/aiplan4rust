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
    /// This method associates a given interned `Literal`—typically representing the filename or source
    /// label—with its corresponding source code content. This association enables diagnostic tools
    /// to render meaningful error messages with spans, line numbers, and context.
    ///
    /// # Arguments
    ///
    /// * `source_id` - An interned `Literal` identifier for the source (e.g., a filename or "stdin").
    /// * `source_content` - The full contents of the source file as a `String`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// diagnostic_manager.add_source(file_id, source_code.to_string());
    /// ```
    ///
    /// # Notes
    ///
    /// - The `source_id` should originate from the same `StringInterner` used across your parsing pipeline.
    /// - The registered content is later used to resolve spans (`Span`) into line/column information and
    ///   to display annotated diagnostics (e.g., underlined errors).
    /// - If a source with the same `Literal` is already registered, this call will overwrite it.
    pub fn add_source(&mut self, source_id: Literal, source_content: String) {
        self.sources.insert(source_id, source_content);
    }

    /// Retrieves the source content associated with a given interned source identifier.
    ///
    /// This method returns the full source text previously registered using [`add_source`],
    /// based on its interned identifier (`source_id`). The `Literal` serves as a unique handle
    /// to avoid storing or passing full string paths repeatedly.
    ///
    /// # Arguments
    ///
    /// * `source_id` - A `Literal` that uniquely identifies a source file or input origin.
    ///
    /// # Returns
    ///
    /// * `Some(&String)` if the source content has been registered for the given `source_id`.
    /// * `None` if no content is associated with the identifier.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(content) = diagnostic_manager.get_source_content(file_id) {
    ///     println!("First 100 characters:\n{}", &content[..100.min(content.len())]);
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - This method is essential for rendering source-level diagnostics with context (e.g., code snippets).
    /// - `source_id` must have been registered beforehand using [`add_source`].
    pub fn get_source_content(&self, source_id: Literal) -> Option<&String> {
        print!("Retrieving source content for: {}", source_id);
        let content = self.sources.get(&source_id);
        print!("{:?}", content);
        content
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

    /// Remaps all [`Ident`] and [`Literal`] values contained in the diagnostics managed by this [`DiagnosticManager`].
    ///
    /// This method is typically used during the **linking phase**, when diagnostics generated from
    /// separate sources (e.g., domain and problem files) are unified. Because each source may have used
    /// its own [`Ident`]s and [`Literal`]s (via separate interners), this remapping aligns all identifiers
    /// to a shared, global interner.
    ///
    /// The remapping ensures that:
    /// - Diagnostic messages refer to the correct unified symbols.
    /// - References in spans and identifiers are consistent after linking.
    /// - Identifier resolution in renderers is accurate.
    ///
    /// # Arguments
    ///
    /// * `idents` - A map from old [`Ident`]s (from source-specific interners) to their global equivalents.
    /// * `literals` - A map from old [`Literal`]s (source identifiers for file/module names) to global equivalents.
    ///
    /// # Behavior
    ///
    /// This method modifies all internal diagnostics in place, applying the given identifier and literal
    /// mappings to each one. If an identifier or literal does not appear in the corresponding map,
    /// it is left unchanged.
    ///
    /// Additionally, it remaps the keys of the internal `sources` map to ensure consistency of source literals.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut manager = DiagnosticManager::default();
    ///
    /// // Assume `old_ident` and `old_literal` came from a specific source file.
    /// let mut id_map = HashMap::new();
    /// id_map.insert(old_ident, global_ident);
    ///
    /// let mut lit_map = HashMap::new();
    /// lit_map.insert(old_literal, global_literal);
    ///
    /// manager.remap(&id_map, &lit_map);
    /// ```
    ///
    /// # Notes
    ///
    /// - This method is intended for post-processing diagnostics before rendering or exporting.
    /// - It is safe to call this method multiple times; repeated calls will apply the map again.
    ///
    /// [`Ident`]: crate::interner::Ident
    /// [`Literal`]: crate::interner::Literal
    /// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
    pub fn remap(&mut self, idents: &HashMap<Ident, Ident>, literals: &HashMap<Literal, Literal>) {
        for diagnostic in &mut self.diagnostics {
            diagnostic.remap(idents, literals);
        }

        // Remap the keys of the `sources` map by applying `remap_literal` on each key.
        let mut new_sources = HashMap::with_capacity(self.sources.len());
        for (mut key, value) in self.sources.drain() {
            key.remap_literal(literals);
            new_sources.insert(key, value);
        }
        self.sources = new_sources;
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
