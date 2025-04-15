use clap::builder::Str;

// Enum pour différents types de diagnostics (erreurs, avertissements, etc.)
pub enum DiagnosticKind {
    UnexpectedToken,
    MissingSemicolon,
    InvalidIndentation,
    CustomError(String),
    // Ajoute d'autres types de diagnostics ici
}

impl DiagnosticKind {
    pub fn code(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken => "E0001".to_string(),
            DiagnosticKind::MissingSemicolon => "E0002".to_string(),
            DiagnosticKind::InvalidIndentation => "E0003".to_string(),
            DiagnosticKind::CustomError(_) => "E0001".to_string()
        }
    }

    // Centraliser le message d'erreur directement dans l'enum
    pub fn message(&self) -> String {
        match self {
            DiagnosticKind::UnexpectedToken => "Unexpected token encountered".to_string(),
            DiagnosticKind::MissingSemicolon => "Missing semicolon".to_string(),
            DiagnosticKind::InvalidIndentation => "Invalid indentation".to_string(),
            DiagnosticKind::CustomError(msg) => msg.to_string(),
        }
    }

    // Ajouter des suggestions, liées au type d'erreur
    pub fn suggestion(&self) -> Option<String> {
        match self {
            DiagnosticKind::MissingSemicolon => Some("Add a semicolon at the end of the statement.".to_string()),
            DiagnosticKind::UnexpectedToken => Some("Check the token syntax.".to_string()),
            DiagnosticKind::InvalidIndentation => Some("Align your code properly.".to_string()),
            DiagnosticKind::CustomError(_) => None,
        }
    }
}
