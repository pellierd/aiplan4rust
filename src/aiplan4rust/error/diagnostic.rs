use crate::aiplan4rust::error::diagnostic_kind::DiagnosticKind;
use crate::aiplan4rust::error::diagnotic_severity::Severity;
use crate::aiplan4rust::parser::Span;

// Structure Diagnostic qui représente un diagnostic complet
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub message: String,
    pub span: Option<Span>,
    pub help_message: Option<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, kind: DiagnosticKind, message: String, span: Option<Span>, help_message: Option<String>) -> Self {
        Diagnostic {
            severity,
            kind,
            message,
            span,
            help_message,
        }
    }

    pub fn display(&self) -> String {
        let severity = match &self.severity {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
            Severity::Help => "Help",
        };

        let mut output = format!("[{}] - {}", severity, self.message);

        if let Some(span) = &self.span {
            output.push_str(&format!("\nAt position: {:?}", span));
        }

        if let Some(help_message) = &self.help_message {
            output.push_str(&format!("\nHelp: {}", help_message));
        }

        output
    }
}
