use std::collections::HashMap;
use std::fmt;
use lalrpop_util::ParseError;
use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::{DiagnosticKind, Provider};
use crate::aiplan4rust::interner::{Ident, Literal};
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
    pub filename: Literal,

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
        filename: Literal,
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
    pub fn filename(&self) -> Literal {
        self.filename
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
    pub fn set_filename(&mut self, filename: Literal) {
        self.filename = filename;
    }

    /// Sets the span for this diagnostic.
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
        let provider_code = self.source.code();

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
            self.source,
            self.filename,
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
