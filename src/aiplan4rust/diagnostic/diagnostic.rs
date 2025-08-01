//! Diagnostic system for parsing, semantic analysis, and compilation phases.
//!
//! This module provides a structured framework for reporting and managing diagnostics
//! such as errors, warnings, and informational messages that occur during the
//! parsing, validation, and compilation of AI planning files.
//!
//! # Overview
//!
//! The diagnostic system is organized into several components:
//!
//! - [`diagnostic`] defines the core `Diagnostic` type and its conversion from parsing errors.
//! - [`kind`] contains a rich set of `Kind` variants to describe different types of issues.
//! - [`severity`] categorizes diagnostics by severity (e.g., error, warning).
//! - [`diagnostic_manager`] manages a collection of diagnostics and associated source files.
//! - [`provider`] distinguishes the origin of a diagnostic (e.g., domain or problem file).
//! - [`renderer`] contains submodules for formatting diagnostics for display, including:
//!   - [`message`] for message construction,
//!   - [`suggestion`] for auto-fix or guidance,
//!   - [`formatting`] utilities to convert internal data to readable strings,
//!   - [`renderer`] for rendering full diagnostics in user-facing form.
//!
//! # Key Concepts
//!
//! - Diagnostics are created during parsing, semantic validation, or symbol resolution.
//! - Each diagnostic carries metadata such as file origin (`Literal`), source span, and severity.
//! - Formatting modules ensure messages are user-friendly and contextual.
//!
//! # Usage Example
//!
//! ```rust
//! use aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, Provider};
//! use aiplan4rust::interner::StringInterner;
//!
//! let kind = DiagnosticKind::InvalidToken;
//! let provider = Provider::Parser;
//! let source = interner.intern_literal("domain.pddl");
//! let span = Span::new(0, 5);
//! let diagnostic = Diagnostic::new(kind, provider, source, span);
//! ```
//!
//! This system enables consistent error reporting across the parsing and compilation pipeline.

use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::{DiagnosticKind, Provider};
use crate::aiplan4rust::interner::{Ident, Literal};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::{FastLineTable, Span};

use std::collections::HashMap;
use std::fmt;
use lalrpop_util::ParseError;

/// Represents a diagnostic message generated during parsing, validation, or compilation.
///
/// A `Diagnostic` describes an issue detected in the source code. It includes metadata
/// about the nature of the issue (`kind`), its origin (`provider`), the file in which
/// it occurred (`source`), and the specific location (`span`) for accurate reporting.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The specific type of diagnostic, such as a syntax error, type mismatch, or unused symbol.
    ///
    /// This defines what kind of issue was detected.
    pub kind: Kind,

    /// Identifies the origin of the diagnostic (e.g., from the domain file, problem file, or unknown).
    ///
    /// Helps distinguish between different inputs or compilation units.
    pub provider: Provider,

    /// An interned identifier representing the source file where the issue occurred.
    ///
    /// This is typically obtained via a `StringInterner` and refers to a file path or logical source name.
    pub source: Literal,

    /// The span within the source file that pinpoints the location of the issue.
    ///
    /// Used for error highlighting and precise reporting (line and column numbers).
    pub span: Span,
}

impl Diagnostic {
    /// Creates a new `Diagnostic` instance containing information about a detected issue.
    ///
    /// # Arguments
    ///
    /// * `kind` - Describes the type of diagnostic (e.g., syntax error, type mismatch).
    /// * `provider` - Indicates the source or subsystem that generated the diagnostic (e.g., Domain, Problem, Parser).
    /// * `source` - A `Literal` identifying the source file where the issue occurred (via the interner).
    /// * `span` - The precise location in the file where the issue is found.
    ///
    /// # Returns
    ///
    /// A new `Diagnostic` ready to be added to the diagnostic manager or displayed.
    pub fn new(
        kind: Kind,
        provider: Provider,
        source: Literal,
        span: Span,
    ) -> Self {
        Diagnostic {
            kind,
            provider,
            source,
            span,
        }
    }

    /// Returns a reference to the kind of diagnostic.
    ///
    /// This includes the structured variant describing the nature of the issue
    /// (e.g., `UnexpectedToken`, `TypeMismatch`, etc.).
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the provider that reported this diagnostic.
    ///
    /// This indicates the origin of the error, such as the domain file, problem file,
    /// or parser infrastructure.
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// Returns the identifier of the source file where this diagnostic occurred.
    ///
    /// The identifier is a `Literal`, typically interned to reduce duplication.
    pub fn source(&self) -> Literal {
        self.source
    }

    /// Returns a reference to the span where the issue was detected.
    ///
    /// The span contains the start and end positions, typically line and column,
    /// allowing precise highlighting or error tracking.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Updates the diagnostic kind with a new value.
    ///
    /// This allows changing the classification or message content of the diagnostic.
    pub fn set_kind(&mut self, kind: Kind) {
        self.kind = kind;
    }

    /// Updates the diagnostic's provider (e.g., Domain, Problem).
    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = provider;
    }

    /// Updates the source file associated with this diagnostic.
    ///
    /// The new value must be a valid `Literal` reference to an interned filename.
    pub fn set_source(&mut self, source: Literal) {
        self.source = source;
    }

    /// Updates the span indicating where this diagnostic applies.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    /// Remaps all `Ident` values inside this diagnostic using a provided identifier mapping.
    ///
    /// This is used to reconcile identifier differences between merged sources, such as
    /// linking a domain and a problem where identifiers may need to be unified or replaced.
    ///
    /// Only the inner [`Kind`] is affected, since it may contain `Ident` values via
    /// usages, declarations, types, or other structures. The other fields (`source`,
    /// `filename`, and `span`) remain unchanged.
    ///
    /// # Arguments
    ///
    /// * `map` - A `HashMap` mapping old `Ident`s to new `Ident`s.
    ///
    /// # Example
    ///
    /// ```
    /// let mut diag = Diagnostic { /* ... */ };
    /// let mut map = HashMap::new();
    /// map.insert(old_id, new_id);
    /// diag.remap(&map);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.kind.remap_idents(map);
    }

    /// Returns a unique diagnostic code string composed of:
    /// - A letter for severity (E, W, I, H)
    /// - A digit for the provider (0-5)
    /// - A two-digit code unique to the kind of diagnostic
    ///
    /// Example: "E01XX" means Error (E), Normalizer (1), code XX for the specific kind.
    pub fn code(&self) -> String {
        // Severity code, e.g. "E"
        let severity_code = self.kind.severity().code();

        // Provider code, e.g. '0'
        let provider_code = self.provider.code();

        // Kind-specific two-character code, e.g. "01"
        // Assume Kind::code() returns &'static str with 2 digits like "01", "05", etc.
        let kind_code = self.kind.code();

        format!("{}{}{}", severity_code, provider_code, kind_code)
    }

    /// Cleans the expected tokens list by removing quotes.
    fn clean_expected(expected: &[String]) -> Vec<String> {
        expected.iter().map(|s| s.replace('"', "")).collect()
    }
}

impl fmt::Display for Diagnostic {
    /// Provides a human-readable display of the diagnostic.
    ///
    /// Example output:
    /// `[Error] Domain at file.pddl:12:5`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[{:?}] {} at {}:{}:{}",
            self.kind,
            self.provider,
            self.source,
            self.span.begin_line(),
            self.span.begin_column()
        )
    }
}

impl<'a> From<(&'a ParseError<usize, Token, LexicalError>, Literal, &'a FastLineTable)> for Diagnostic {
    /// Converts a LALRPOP `ParseError` into a structured `Diagnostic`, enriched with
    /// source span and interner-based file context.
    ///
    /// This implementation maps a `ParseError` (produced by the parser), along with
    /// a file identifier (`Literal`) and a `FastLineTable`, into a `Diagnostic`
    /// value suitable for reporting. It resolves parser-specific errors
    /// into diagnostic kinds and calculates source spans for accurate positioning.
    ///
    /// # Arguments
    ///
    /// The input is a tuple consisting of:
    /// - `&ParseError<usize, Token, LexicalError>`: the parse error to convert.
    /// - `Literal`: the interned identifier of the source file where the error occurred.
    /// - `&FastLineTable`: used to convert byte offsets into source code spans (line/column).
    ///
    /// # Returns
    ///
    /// A `Diagnostic` instance representing the error, with its kind, source location,
    /// and source file identifier.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let diagnostic = Diagnostic::from((&parse_error, file_id, &line_table));
    /// eprintln!("{}", diagnostic);
    /// ```
    ///
    /// # Notes
    ///
    /// - The `Literal` is not a file path but an interned handle to the file identifier.
    /// - For `User`-defined errors, a fallback empty span is used (position 0).
    /// - Expected token names are cleaned before being included in the message.
    fn from(
        value: (&'a ParseError<usize, Token, LexicalError>, Literal, &'a FastLineTable),
    ) -> Self {
        let (error, source, fast_line_table) = value;

        match error {
            // Handles unexpected tokens by including the token and expected set
            ParseError::UnrecognizedToken {
                token: (start, t, end),
                expected,
            } => {
                // Clean expected tokens by removing quotes for better message display
                let clean_expected = Diagnostic::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        token: t.to_string(),
                        expected: clean_expected,
                    },
                    Provider::Parser,
                    source,
                    // Calculate the span using FastLineTable for accurate error location
                    fast_line_table.get_span(*start, *end),
                )
            }
            // Handles invalid token errors at a specific location
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*location, *location),
                )
            }
            // Handles user-defined errors with arbitrary messages
            ParseError::User { error } => {
                let content = error.to_string();
                Diagnostic::new(
                    DiagnosticKind::User {
                        message: content,
                    },
                    Provider::Parser,
                    source,
                    // No span information available, use empty span (0,0)
                    fast_line_table.get_span(0, 0),
                )
            }
            // Handles unexpected EOF errors and lists expected tokens
            ParseError::UnrecognizedEof { location, expected } => {
                let clean_expected = Diagnostic::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof { expected: clean_expected },
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*location, *location),
                )
            }
            // Handles extra token errors, providing the token string
            ParseError::ExtraToken {
                token: (start, t, end),
            } => {
                Diagnostic::new(
                    DiagnosticKind::ExtraToken {
                        token: t.to_string(),
                    },
                    Provider::Parser,
                    source,
                    fast_line_table.get_span(*start, *end),
                )
            }
        }
    }
}
