use std::fmt;
use clap::builder::Str;
use logos::Source;
use crate::aiplan4rust::error::DiagnosticSeverity;

// Enum pour différents types de diagnostics (erreurs, avertissements, etc.)
#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticKind<'a> {
    UnexpectedToken,
    UnrecognizedToken {
        token: String,
        expected: &'a Vec<String>,
    },
    CustomError(String),
}

impl<'a> DiagnosticKind<'a> {
    pub fn code(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken => "E0001".to_string(),
            DiagnosticKind::UnrecognizedToken{ .. } => "E0002".to_string(),
            DiagnosticKind::CustomError(_) => "E0001".to_string()
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken => "Unrecognized token encountered".to_string(),
            DiagnosticKind::UnrecognizedToken {token, ..} => format!("Unexpected token '{}' encountered", token),
            DiagnosticKind::CustomError(msg) => msg.to_string(),
        }
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            DiagnosticKind::UnexpectedToken => DiagnosticSeverity::Error,
            DiagnosticKind::UnrecognizedToken { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::CustomError(_) => DiagnosticSeverity::Error,
        }
    }
    pub fn suggestion(&self) -> Option<String> {
        match self {
            DiagnosticKind::UnexpectedToken => {
                Some("Add a semicolon at the end of the statement.".to_string())
            }
            DiagnosticKind::UnrecognizedToken { token, expected } => {
                if expected.is_empty() {
                    Some("Check the token syntax.".to_string())
                } else if expected.len() == 1 {
                    Some(format!(
                        "Expected token: `{}`.",
                        expected[0]
                    ))
                } else {
                    Some(format!(
                        "Expected one of the following tokens: {}.",
                        expected.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(", ")
                    ))
                }
            }
            DiagnosticKind::CustomError(_) => None,
        }
    }
}

impl<'a> fmt::Display for DiagnosticKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        let message = self.message();
        let severity = self.severity();
        let suggestion = self.suggestion();

        match suggestion {
            Some(sugg) => write!(
                f,
                "[{}] ({}) {}. Suggestion: {}",
                code,
                severity,
                message,
                sugg
            ),
            None => write!(f, "[{}] ({}) {}", code, severity, message),
        }
    }
}
