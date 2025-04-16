use std::fmt;
use crate::aiplan4rust::error::diagnostic_kind::DiagnosticKind;
use crate::aiplan4rust::error::DiagnosticSource;
use crate::aiplan4rust::error::diagnotic_severity::DiagnosticSeverity;
use crate::aiplan4rust::parser::Span;

// Structure Diagnostic qui représente un diagnostic complet
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic<'a> {
    pub kind: DiagnosticKind<'a>,
    pub source: DiagnosticSource,
    pub filename: String,
    pub span: Span,
}

impl<'a> Diagnostic<'a> {
    pub fn new(kind: DiagnosticKind<'a>, source: DiagnosticSource, filename: String, span: Span) -> Self {
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
    pub fn set_kind(&mut self, kind: DiagnosticKind<'a>) {
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

impl<'a> fmt::Display for Diagnostic<'a> {
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
