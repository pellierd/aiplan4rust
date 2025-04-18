use std::fmt;
use clap::builder::Str;
use logos::Source;
use crate::aiplan4rust::error::DiagnosticSeverity;
use crate::aiplan4rust::parser::elements::Requirement;

// Enum pour différents types de diagnostics (erreurs, avertissements, etc.)
#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedToken {
        token: String,
        expected: Vec<String>,
    },
    UnexpectedEof {
        expected: Vec<String>,
    },
    InvalidToken,
    ExtraToken {
        token: String,
    },
    DuplicatedRequirementDeclaration {
        requirement: Requirement
    },
    DuplicatedTypeDeclaration {
        ty: String,
    },
    CustomError(String),
}

impl DiagnosticKind {
    pub fn code(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken{ .. } => "E0001".to_string(),
            DiagnosticKind::UnexpectedEof{ .. } => "E0002".to_string(),
            DiagnosticKind::InvalidToken => "E0003".to_string(),
            DiagnosticKind::ExtraToken{ .. } => "E0004".to_string(),
            DiagnosticKind::DuplicatedRequirementDeclaration { .. } => "E0005".to_string(),
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => "E0006".to_string(),
            DiagnosticKind::CustomError(_) => "E0001".to_string()
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken { token, .. } => {
                format!("Unexpected token '{}'.", token)
            }
            DiagnosticKind::UnexpectedEof { .. } => {
                "Unexpected end of input (EOF).".to_string()
            }
            DiagnosticKind::InvalidToken => {
                "Unrecognized or malformed token.".to_string()
            }
            DiagnosticKind::ExtraToken { token } => {
                format!("Unexpected extra token '{}'.", token)
            }
            DiagnosticKind::DuplicatedRequirementDeclaration { requirement } => {
                format!("Requirement '{}' is declared more than once.", requirement)
            }
            DiagnosticKind::DuplicatedTypeDeclaration { ty } => {
                format!("Type '{}' is declared multiple times.", ty)
            }
            DiagnosticKind::CustomError(msg) => msg.to_string(),
        }
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            DiagnosticKind::UnexpectedToken { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::UnexpectedEof { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::InvalidToken => DiagnosticSeverity::Error,
            DiagnosticKind::ExtraToken { .. } => DiagnosticSeverity::Error,
            DiagnosticKind::CustomError(_) => DiagnosticSeverity::Error,
            DiagnosticKind::DuplicatedRequirementDeclaration { .. } => DiagnosticSeverity::Warning,
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => DiagnosticSeverity::Warning,
        }
    }
    pub fn suggestion(&self) -> Option<String> {
        match self {
            DiagnosticKind::UnexpectedToken { expected, .. }
            | DiagnosticKind::UnexpectedEof { expected } => {
                Self::format_expected_message(expected)
            }
            DiagnosticKind::ExtraToken { .. } => {
                Some("Extra token detected. Check for unnecessary symbols or misplaced characters.".to_string())
            }
            DiagnosticKind::InvalidToken => {
                Some("Make sure there are no typos or invalid characters.".to_string())
            }
            DiagnosticKind::DuplicatedRequirementDeclaration { .. } => {
                Some("This requirement is already declared. You can safely remove the duplicate.".to_string())
            }
            DiagnosticKind::DuplicatedTypeDeclaration { .. } => {
                Some("This type is already declared. Consider removing the duplicate.".to_string())
            }
            DiagnosticKind::CustomError(_) => None,
        }
    }

    fn format_expected_message(expected: &[String]) -> Option<String> {
        match expected.len() {
            0 => Some("Unexpected input. Please verify the syntax near this token.".to_string()),
            1 => Some(format!("Expected token: `{}`.", expected[0])),
            _ => Some(format!(
                "Expected one of the following tokens: {}.",
                Self::join_expected_tokens(expected)
            )),
        }
    }

    fn join_expected_tokens(expected: &[String]) -> String {
        expected
            .iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for DiagnosticKind {
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
