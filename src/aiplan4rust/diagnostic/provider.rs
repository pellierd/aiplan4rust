//! Module defining the `Provider` enum which represents the different
//! components or stages in a processing pipeline responsible for providing
//! diagnostics or other services.
//!
//! This module defines the `Provider` enum that identifies the different
//! components or stages in a processing pipeline, responsible for generating
//! diagnostics or other related services.
//!
//! # Variants:
//! - `Parser`: The component responsible for syntactic analysis.
//! - `Normalizer`: The component that normalizes or transforms data structures.
//! - `Analyzer`: The component performing semantic analysis or other processing.
//! - `Linker`: The component that links and integrates different parts or modules.
//! - `LirBuilder`: The component responsible for building the low-level intermediate representation (LIR).
//! - `Grounder`: The component responsible for grounding or instantiating abstract representations into concrete forms.
//!
//! # Usage example:
//! ```
//! use crate::aiplan4rust::diagnostic::Provider;
//!
//! let prov = Provider::Parser;
//! println!("Current provider: {}", prov); // Prints "Current provider: Parser"
//! ```

use std::fmt;

/// Enum representing the different providers or components in a processing pipeline.
///
/// This enum identifies the source or stage responsible for generating diagnostics
/// or performing a specific processing task.
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Provider {
    /// The parser component responsible for syntactic analysis.
    Parser,

    /// The normalizer component responsible for normalizing or transforming data structures.
    Normalizer,

    /// The analyzer component responsible for semantic or other analysis.
    Analyzer,

    /// The linker component responsible for linking and integrating different parts or modules.
    Linker,

    /// The LIR (Lifted Intermediate Representation) builder component responsible for
    /// constructing the lifted intermediate representation used in further compilation stages.
    LirGenerator,

    /// The grounder component responsible for grounding or instantiating abstract representations into concrete forms.
    Grounder,
}

impl Provider {
    /// Returns a single-digit string code representing the provider.
    ///
    /// For example:
    /// - `"0"` for Parser
    /// - `"1"` for Normalizer
    /// - `"2"` for Analyzer
    /// - `"3"` for Linker
    /// - `"4"` for LirGenerator
    /// - `"5"` for Grounder
    pub fn code(&self) -> &'static str {
        match self {
            Provider::Parser => "0",
            Provider::Normalizer => "1",
            Provider::Analyzer => "2",
            Provider::Linker => "3",
            Provider::LirGenerator => "4",
            Provider::Grounder => "5",
        }
    }
}


impl fmt::Display for Provider {
    /// Formats the `Provider` enum as a human-readable string.
    ///
    /// # Example
    /// ```
    /// use crate::aiplan4rust::diagnostic::Provider;
    ///
    /// let provider = Provider::Lexer;
    /// assert_eq!(format!("{}", provider), "Lexer");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source_str = match self {
            Provider::Parser => "Lexer",
            //Provider::Parser => "Parser",
            Provider::Normalizer => "Normalizer",
            Provider::Analyzer => "Analyzer",
            Provider::Linker => "Linker",
            Provider::LirGenerator => "LirBuilder",
            Provider::Grounder => "Grounder",
        };
        write!(f, "{}", source_str)
    }
}
