use std::fmt;
use lalrpop_util::ParseError;
use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::{DiagnosticKind, Provider};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::{FastLineTable, Span};

/// Represents a diagnostic generated during parsing, validation, or compilation.
///
/// A `Diagnostic` contains information about an issue found in the source file,
/// including the kind of problem, its source (e.g., domain or problem file),
/// the file in which it occurred, and the precise location (`Span`).
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The kind of diagnostic (e.g., Error, Warning, Info).
    pub kind: Kind,

    /// The source of the diagnostic (Domain, Problem, or Unknown).
    pub source: Provider,

    /// The name of the file where the diagnostic occurred.
    pub filename: String,

    /// The span (line/column information) where the diagnostic applies.
    pub span: Span,
}

impl Diagnostic {
    /// Creates a new `Diagnostic` instance.
    ///
    /// # Arguments
    ///
    /// * `kind` - The severity and category of the diagnostic.
    /// * `source` - The origin of the diagnostic (domain/problem).
    /// * `filename` - The path of the file in which the diagnostic was found.
    /// * `span` - The position in the file where the issue occurred.
    pub fn new(
        kind: Kind,
        source: Provider,
        filename: String,
        span: Span,
    ) -> Self {
        Diagnostic {
            kind,
            source,
            filename,
            span,
        }
    }

    /// Returns a reference to the kind of this diagnostic.
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns a reference to the source of this diagnostic.
    pub fn source(&self) -> &Provider {
        &self.source
    }

    /// Returns a reference to the filename where the diagnostic occurred.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Returns a reference to the span associated with this diagnostic.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Sets the kind of this diagnostic.
    pub fn set_kind(&mut self, kind: Kind) {
        self.kind = kind;
    }

    /// Sets the source of this diagnostic.
    pub fn set_source(&mut self, source: Provider) {
        self.source = source;
    }

    /// Sets the filename associated with this diagnostic.
    pub fn set_filename<S: Into<String>>(&mut self, filename: S) {
        self.filename = filename.into();
    }

    /// Sets the span for this diagnostic.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}

impl Diagnostic {
    /// Converts a `ParseError` into a `Diagnostic`.
    ///
    /// Formats the error message based on the type_checker of `ParseError` encountered (e.g., unrecognized token, invalid token, etc.)
    /// and includes the source location and file path (if available).
    ///
    /// # Arguments
    /// * `error` - The `ParseError` to be converted, containing the error details.
    /// * `file_path` - Optional path to the source file as a string slice.
    /// * `fast_line_table` - A `FastLineTable` instance for span calculation.
    ///
    /// # Returns
    /// A `Diagnostic` representing the parse error.
    pub fn from_parse_error(
        error: &ParseError<usize, Token, LexicalError>,
        file_path: Option<&str>,
        fast_line_table: &FastLineTable,
    ) -> Self {
        let file_path = file_path.unwrap_or("").to_string();

        match error {
            ParseError::UnrecognizedToken {
                token: (start, t, end),
                expected,
            } => {
                let clean_expected = Self::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        token: t.to_string(),
                        expected: clean_expected,
                    },
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(*start, *end),
                )
            }
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(*location, *location),
                )
            }
            ParseError::User { error } => {
                let content = error.to_string();
                Diagnostic::new(
                    DiagnosticKind::CustomError(content),
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(0, 0),
                )
            }
            ParseError::UnrecognizedEof { location, expected } => {
                let clean_expected = Self::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof { expected: clean_expected },
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(*location, *location),
                )
            }
            ParseError::ExtraToken {
                token: (start, t, end),
            } => {
                Diagnostic::new(
                    DiagnosticKind::ExtraToken {
                        token: t.to_string(),
                    },
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(*start, *end),
                )
            }
        }
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
            self.source,
            self.filename,
            self.span.begin_line(),
            self.span.begin_column()
        )
    }
}
