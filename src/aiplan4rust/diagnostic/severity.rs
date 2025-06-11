use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Error,      // Erreur fatale, arrêt du processus
    Warning,    // Avertissement, mais le programme peut continuer
    Info,       // Information, utile mais non essentiel
    Help,       // Aide contextuelle pour guider l'utilisateur
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
            Severity::Help => "Help",
        };
        write!(f, "{}", severity_str)
    }
}
