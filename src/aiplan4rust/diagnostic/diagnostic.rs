use std::collections::HashMap;
use std::fmt;
use lalrpop_util::ParseError;
use crate::aiplan4rust::diagnostic::kind::Kind;
use crate::aiplan4rust::diagnostic::{DiagnosticKind, Provider};
use crate::aiplan4rust::interner::Ident;
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
}

impl Diagnostic {
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

impl<'a> From<(&'a ParseError<usize, Token, LexicalError>, Option<&'a str>, &'a FastLineTable)> for Diagnostic {
    /// Converts a LALRPOP `ParseError` along with optional file path and a `FastLineTable`
    /// into a `Diagnostic` struct, which holds detailed error information suitable
    /// for reporting and displaying to the user.
    ///
    /// This implementation maps different variants of `ParseError` to appropriate
    /// diagnostic kinds and computes the source span for error highlighting.
    ///
    /// # Arguments
    /// * `value` - A tuple containing:
    ///     - Reference to the `ParseError` to convert.
    ///     - Optional file path as a string slice.
    ///     - Reference to a `FastLineTable` used for calculating source spans.
    ///
    /// # Returns
    /// A `Diagnostic` instance representing the detailed error.
    fn from(
        value: (&'a ParseError<usize, Token, LexicalError>, Option<&'a str>, &'a FastLineTable),
    ) -> Self {
        let (error, file_path_opt, fast_line_table) = value;
        // Use provided file path or default to empty string if none given
        let file_path = file_path_opt.unwrap_or("").to_string();

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
                    Provider::Lexer,
                    file_path,
                    // Calculate the span using FastLineTable for accurate error location
                    fast_line_table.get_span(*start, *end),
                )
            }
            // Handles invalid token errors at a specific location
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    Provider::Lexer,
                    file_path,
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
                    Provider::Lexer,
                    file_path,
                    // No span information available, use empty span (0,0)
                    fast_line_table.get_span(0, 0),
                )
            }
            // Handles unexpected EOF errors and lists expected tokens
            ParseError::UnrecognizedEof { location, expected } => {
                let clean_expected = Diagnostic::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof { expected: clean_expected },
                    Provider::Lexer,
                    file_path,
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
                    Provider::Lexer,
                    file_path,
                    fast_line_table.get_span(*start, *end),
                )
            }
        }
    }
}
