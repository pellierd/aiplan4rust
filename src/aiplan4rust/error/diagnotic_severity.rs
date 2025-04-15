#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Error,      // Erreur fatale, arrêt du processus
    Warning,    // Avertissement, mais le programme peut continuer
    Info,       // Information, utile mais non essentiel
    Help,       // Aide contextuelle pour guider l'utilisateur
}