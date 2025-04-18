use std::collections::HashMap;
use crate::aiplan4rust::error::{Diagnostic, DiagnosticKind};


pub struct DiagnosticManager {
    diagnostics: Vec<Diagnostic>,
    sources: HashMap<String, String>,
}
impl DiagnosticManager {
    pub fn new() -> Self {
        DiagnosticManager { diagnostics: Vec::new(), sources: HashMap::new() }
    }

    pub fn add_source(&mut self, filename: String, source: String) {
        self.sources.insert(filename, source);
    }
    pub fn get_source(&self, filename: &str) -> Option<&String> {
        self.sources.get(filename)
    }
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    pub fn add_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }


    pub fn has_diagnotics_of_kind(&self, kind: DiagnosticKind) -> bool {
        self.diagnostics.iter().any(|e| *e.kind() == kind)
    }

    pub fn reset(&mut self) {
        self.diagnostics.clear();
    }

    pub fn add_errors_from(&mut self, other: &DiagnosticManager) {
        self.diagnostics.extend(other.diagnostics.iter().cloned());
        self.sources.extend(other.sources.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn display_all(&self) {
        let mut sorted_errors = self.diagnostics.clone();

        // Sort errors by line number, then by column number if line numbers are equal
        sorted_errors.sort_by_key(|e| (e.span().begin_line(), e.span().begin_column()));

        for diagnostic in sorted_errors {
            println!("{}", diagnostic);
        }
    }
}
