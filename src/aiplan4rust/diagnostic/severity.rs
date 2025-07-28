//! Module defining the `Severity` enum representing the level of severity
//! for diagnostic messages such as errors, warnings, informational notes,
//! and help messages.
//!
//! Ce module contient l'énumération `Severity` qui catégorise
//! le degré d'importance des messages diagnostiques générés par
//! un programme.
//!
//! # Variants / Variantes :
//! - `Error` : Erreur fatale entraînant généralement l'arrêt du processus.
//! - `Warning` : Avertissement signalant un problème potentiel, mais le programme continue.
//! - `Info` : Message informatif utile mais non essentiel.
//! - `Help` : Message d'aide contextuelle pour guider l'utilisateur.
//!
//! # Usage example / Exemple d'utilisation :
//! ```
//! use your_crate::Severity;
//!
//! let sev = Severity::Warning;
//! println!("Severity level: {}", sev); // Affiche "Severity level: Warning"
//! ```

use std::fmt;

/// Represents the severity level of a diagnostic message or event.
///
/// This enum categorizes the importance and impact of messages that
/// might be produced by a program, such as errors, warnings, informational
/// notes, or contextual help.
///
/// # Variants
///
/// - `Error`: A fatal error that typically stops the process or requires immediate attention.
/// - `Warning`: A warning indicating a potential issue; the program can usually continue.
/// - `Info`: Informational message that is useful but not essential.
/// - `Help`: Contextual help or guidance to assist the user.
///
/// # Examples
///
/// ```
/// use your_crate::Severity;
///
/// let sev = Severity::Warning;
/// println!("Severity level: {}", sev); // prints "Severity level: Warning"
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    /// Fatal error that halts execution or demands immediate fix.
    Error,

    /// Warning indicating a possible problem, but execution can continue.
    Warning,

    /// Informational message that provides useful insights but no direct impact.
    Info,

    /// Helpful message intended to guide or assist the user in resolving issues.
    Help,
}

impl fmt::Display for Severity {
    /// Formats the `Severity` enum as a user-friendly string.
    ///
    /// This implementation converts each variant into a capitalized string
    /// representation, suitable for logging, displaying in user interfaces,
    /// or diagnostic output.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt::Display;
    /// use your_crate::Severity;
    ///
    /// let severity = Severity::Error;
    /// assert_eq!(severity.to_string(), "Error");
    /// ```
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
