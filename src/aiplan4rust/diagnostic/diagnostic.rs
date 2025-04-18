use std::fmt;
use crate::aiplan4rust::diagnostic::diagnostic_kind::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticSource;
use crate::aiplan4rust::parser::Span;

// Structure Diagnostic qui représente un diagnostic complet
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub source: DiagnosticSource,
    pub filename: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(kind: DiagnosticKind, source: DiagnosticSource, filename: String, span: Span) -> Self {
        Diagnostic {
            kind,
            source,
            filename,
            span,
        }
    }

    pub fn kind(&self) -> &DiagnosticKind {
        &self.kind
    }

    pub fn source(&self) -> &DiagnosticSource {
        &self.source
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn span(&self) -> &Span {
        &self.span
    }
    pub fn set_kind(&mut self, kind: DiagnosticKind) {
        self.kind = kind;
    }

    pub fn set_source(&mut self, source: DiagnosticSource) {
        self.source = source;
    }

    pub fn set_filename<S: Into<String>>(&mut self, filename: S) {
        self.filename = filename.into();
    }

    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[{:?}] {} at {}:{}:{}",
            self.kind,
            self.source,
            self.filename,
            self.span.begin_line(),
            self.span.begin_column()
        )
    }
}
