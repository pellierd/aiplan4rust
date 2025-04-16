use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticSeverity {
    Error,      // Erreur fatale, arrêt du processus
    Warning,    // Avertissement, mais le programme peut continuer
    Info,       // Information, utile mais non essentiel
    Help,       // Aide contextuelle pour guider l'utilisateur
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self {
            DiagnosticSeverity::Error => "Error",
            DiagnosticSeverity::Warning => "Warning",
            DiagnosticSeverity::Info => "Info",
            DiagnosticSeverity::Help => "Help",
        };
        write!(f, "{}", severity_str)
    }
}
