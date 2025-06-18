use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::syntax::lexer::LexicalError;
use crate::aiplan4rust::syntax::parser_result::ParserResult;
use crate::aiplan4rust::syntax::grammar::HDDLParser;
use crate::aiplan4rust::syntax::grammar::PDDLParser;
use crate::aiplan4rust::syntax::{Language, StringInterner};
use crate::aiplan4rust::syntax::Span;

use lalrpop_util::ErrorRecovery;
use lalrpop_util::ParseError;

use std::mem;
use std::time::SystemTime;
use crate::aiplan4rust::syntax::int_ast::{IntAst, IntAstNode};

const AVG_LINE_LENGTH: usize = 80;

#[derive(Debug)]
/// A structure for analyzing the syntax of PDDL expressions.
///
/// The `SyntaxAnalyzer` is responsible for parsing and validating PDDL expressions from
/// a given source string. It works with an optional path for file-based sources and an
/// error manager to handle any parsing errors.
///
/// # Fields
/// - `source`: A reference to the source string containing the PDDL expression to analyze.
/// - `path`: An optional `PathBuf` representing the file path from which the source is read.
/// - `pddl_fragment`: The `PDDLFragment` that represents the parsed expression.
/// - `error_manager`: A mutable reference to the `ErrorManager` for managing parsing errors.
///
/// # Example
/// ```rust
/// use aiplan4rust::aiplan4rust::syntax::syntax::PDDLFragment;
/// let source = "(define (problem test) ...)";
/// let mut error_manager = ErrorManager::new();
/// let pddl_expression = PDDLFragment::Domain; // Example expression
/// let syntax_analyzer = SyntaxAnalyzer {
///     source,
///     path: None,
///     pddl_fragment,
///     error_manager: &mut error_manager,
/// };
/// ```
pub struct Parser<'a> {
    source_name: Option<&'a str>,
    source: Option<&'a str>,
    fast_line_table: FastLineTable,
    diagnostic_manager: DiagnosticManager,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance.
    ///
    /// # Returns
    /// Returns a new instance of `Parser`.
    pub fn new() -> Self {
        Self {
            source_name: None,
            source: None,
            fast_line_table: FastLineTable::default(),
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the `ErrorManager` associated with this instance.
    ///
    /// The `ErrorManager` stores and manages errors encountered during processing.
    /// This function allows access to the error manager for querying or handling errors.
    ///
    /// # Returns
    /// A reference to the `ErrorManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// This function parses a PDDL or HDDL file and returns a `SyntaxTree` and handling errors.
    ///
    /// # Arguments
    ///
    /// * `filename`: The name of the source file being parsed, typically used for error reporting.
    /// * `source`: The source code of the PDDL or HDDL file as a string to be parsed.
    /// * `language`: Specifies which language to use for parsing the source code (either PDDL or
    ///   HDDL).
    ///
    /// # Returns
    ///
    /// * `Result<ParserResult, ParserInternalError>`:
    ///   - If parsing is successful, returns a `ParserResult` containing the generated `SyntaxTree`
    ///   and any errors encountered during parsing.
    ///   - If parsing fails due to lexical or parsing errors, returns an internal error with
    ///   details about the failure.
    ///
    /// # Error Handling
    ///
    /// The function handles both lexical errors (such as invalid tokens) and parsing errors (such
    /// as invalid grammar). It processes any errors and returns them as part of the `ParserResult`
    /// if parsing was unsuccessful.
    ///
    /// # Example
    ///
    /// ```rust
    /// let result = parser.parse("example.pddl", source_code, Language::PDDL);
    /// match result {
    ///     Ok(parser_result) => {
    ///         // Process the resulting syntax tree
    ///     },
    ///     Err(error) => {
    ///         // Handle internal parsing error
    ///     }
    /// }
    /// ```
    pub fn parse(
        &mut self,
        source_name: &'a str,
        source: &'a str,
        language: &Language,
    ) -> Result<ParserResult, ParserInternalError> {
        // Store temporary references to the filename and source for later use
        self.source_name = Some(source_name);
        self.source = Some(source);

        // Initialize a vector to store LALRPOP errors that may occur during parsing
        let mut larlpop_errors = Vec::new();

        // Create a lexer from the provided source code
        let lexer = Lexer::new(source);

        self.diagnostic_manager.add_source(source_name.to_string(), source.to_string());

        let mut context = StringInterner::new();

        // Attempt to parse the source code according to the language specified
        let parse_result = match language {
            Language::PDDL => PDDLParser::new().parse(&mut context, &mut larlpop_errors, lexer),
            Language::HDDL => HDDLParser::new().parse(&mut context, &mut larlpop_errors, lexer),
        };

        // Create a `FastLineTable` with an interval for coarse indexing.
        self.fast_line_table = FastLineTable::new(source);

        // Handle any syntax errors that were collected during parsing
        self.handle_syntax_errors(&larlpop_errors, source);

        if self
            .diagnostic_manager()
            .has_diagnotics_of_severity(Severity::Error)
        {
            Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
        } else {
            match parse_result {
                Ok(mut root) => {
                    if self
                        .diagnostic_manager()
                        .has_diagnotics_of_severity(Severity::Error)
                    {
                        Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                    } else {
                        self.init_ast_span(&mut root);
                        let ast =
                            IntAst::new(root, context, source_name.to_string(), SystemTime::now());
                        Ok(ParserResult::new(
                            Some(ast),
                            mem::take(&mut self.diagnostic_manager),
                        ))
                    }
                }
                Err(e) => {
                    let error = self.to_parser_error(
                        &e,
                        source,
                        Some(source_name),
                    );
                    self.diagnostic_manager.add_diagnostic(error);
                    Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                },
            }
        }
    }

    /// Handles the syntax errors produced by the LALRPOP aiplan4rust.
    /// For each error, a `ParserError` is created and added to the error manager.
    ///
    /// # Arguments
    /// * `larlpop_errors`: A list of errors produced by the LALRPOP aiplan4rust.
    /// * `source`: The source code to reference when generating error messages.
    fn handle_syntax_errors(
        &mut self,
        larlpop_errors: &[ErrorRecovery<usize, Token, LexicalError>],
        source: &'a str,
    ) {
        for larlpop_error in larlpop_errors {
            // Convert each LALRPOP error into a ParserError and add it to the error manager
            let parser_error = self.to_parser_error(&larlpop_error.error, source, self.source_name);
            self.diagnostic_manager.add_diagnostic(parser_error);
        }
    }

    /// Recursively sets the start and end positions (line, column) for each AST node.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to an AST node.
    fn init_ast_span(&self, ast: &mut IntAstNode) {
        // Compute and set the start position of the current AST node
        let (line, column) = self.fast_line_table.get_position(ast.start_offset());
        ast.set_start_position(line, column);

        // Compute and set the end position of the current AST node
        let (line, column) = self.fast_line_table.get_position(ast.end_offset());
        ast.set_end_position(line, column);

        // Recursively process all child nodes of the current AST node
        for child in ast.children_mut() {
            self.init_ast_span(child);
        }
    }

    /// Finds the line and column of a character position in a string.
    ///
    /// This function computes the line and column numbers corresponding to a specific
    /// character position (`position`) within the provided `input` string. It assumes
    /// that lines are separated by newline characters (`\n`), with line and column
    /// numbering starting at 1.
    ///
    /// # Parameters
    /// - `offset`: The zero-based index of the character in the string whose line and column are to
    ///     be determined.
    /// - `source`: A reference to the input string where the character position is located.
    ///
    /// # Returns
    /// A tuple `(usize, usize)` where:
    /// - The first element is the line number (starting from 1).
    /// - The second element is the column number (starting from 1).
    ///
    /// # Example
    /// ```rust
    /// let input = "Hello\nRustaceans!";
    /// let offset = 8; // The character 'R' in "Rustaceans!"
    /// let (line, column) = get_position(offset, input);
    /// assert_eq!((line, column), (2, 1)); // 'R' is on line 2, column 1
    /// ```
    ///
    /// # Notes
    /// - If `offset` is greater than the length of the string, the function
    ///   will return the line and column corresponding to the end of the string.
    /// - The function handles multiline input and correctly resets the column
    ///   count after encountering a newline.
    ///
    /// # Panics
    /// This function does not explicitly panic but assumes that the `offset` is within
    /// the range of valid indices for the string. Out-of-range values may result in unexpected
    /// behavior.
    #[allow(dead_code)]
    fn get_position(&self, offset: usize, source: &str) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for (index, ch) in source.chars().enumerate() {
            if index == offset {
                break;
            }
            match ch {
                '\n' => {
                    line += 1;
                    column = 1;
                }
                _ => {
                    column += 1;
                }
            }
        }
        (line, column)
    }

    /// Converts a `ParseError` into a `ParsingError`.
    ///
    /// This function takes a `ParseError` and converts it into a `ParsingError`, which can be used
    /// for logging or error reporting. It formats the error message based on the type of `ParseError`
    /// encountered (e.g., unrecognized token, invalid token, etc.) and includes the source location and
    /// file path (if available).
    ///
    /// # Arguments
    /// * `error` - The `ParseError` to be converted, containing the error details.
    /// * `source` - The source code as a string, used to get the position of the error.
    /// * `file_path` - An optional `PathBuf` representing the file path of the source.
    ///
    /// # Returns
    /// A `ParsingError` with the formatted error message, type, and location.
    ///
    /// # Example
    /// ```
    /// let parser_error = self.to_parser_error(&parse_error, &source_code, Some(file_path));
    /// ```
    fn to_parser_error(
        &self,
        error: &ParseError<usize, Token, LexicalError>,
        source: &str,
        file_path: Option<&str>,
    ) -> Diagnostic {
        let file_path = file_path.unwrap().to_string();
        match error {
            ParseError::UnrecognizedToken {
                token: (start, t, end),
                expected,
            } => {
                let clean_expected= Self::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        token: t.to_string(),
                        expected:  clean_expected},
                    Provider::Lexer,
                    file_path,
                    self.fast_line_table.get_span(*start, *end),
                )
            }
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    Provider::Lexer,
                    file_path,
                    self.fast_line_table.get_span(*location, *location),
                )
            }
            ParseError::User { error } => {
                let content = error.to_string();
                Diagnostic::new(
                    DiagnosticKind::CustomError(content),
                    Provider::Lexer,
                    file_path,
                    self.fast_line_table.get_span(0, 0),
                )
            }
            ParseError::UnrecognizedEof { location, expected } => {
                let clean_expected= Self::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof {
                        expected:  clean_expected},
                    Provider::Lexer,
                    file_path,
                    self.fast_line_table.get_span(*location, *location),
                )
            }
            ParseError::ExtraToken {
                token: (start, t, end),
            } => {
                Diagnostic::new(
                    DiagnosticKind::ExtraToken {
                        token:  t.to_string(),
                    },
                    Provider::Lexer,
                    file_path,
                    self.fast_line_table.get_span(*start, *end),
                )
            }
        }
    }
    fn clean_expected(expected: &[String]) -> Vec<String> {
        expected.iter()
            .map(|s| s.replace('"', ""))
            .collect()
    }
}

/// A fast line table for efficiently mapping byte offsets to line and column numbers.
/// It uses a coarse index to accelerate lookups.
///
/// # Fields
/// - `line_starts`: A vector storing the starting byte offset of each line.
/// - `coarse_index`: A vector storing precomputed offsets and their corresponding line numbers
///   at intervals of `k` lines to speed up lookups.
/// - `k`: The interval for the pre-index (determines how frequently the coarse index stores values).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct FastLineTable {
    line_starts: Vec<usize>,
    coarse_index: Vec<(usize, usize)>, // (Offset, Line number) every K lines
}

impl FastLineTable {
    /// Creates a new `FastLineTable` with an automatically chosen interval `k`.
    ///
    /// The interval is computed based on the number of lines in the source,
    /// aiming to balance lookup speed and memory usage.
    ///
    /// # Arguments
    /// - `source`: The full input source code as a string slice.
    ///
    /// # Returns
    /// A `FastLineTable` with dynamically tuned indexing.
    pub fn new(source: &str) -> Self {
        let total_lines = bytecount::count(source.as_bytes(), b'\n') + 1;
        let k = std::cmp::max(10, total_lines / 100);
        Self::with_capacity(source, k)
    }

    /// Creates a new `FastLineTable` using a manually specified indexing interval `k`.
    ///
    /// A lower `k` gives faster lookups but increases memory usage. A higher `k` reduces
    /// memory usage but may slow down lookup times. Typical values range from 50 to 500.
    ///
    /// # Arguments
    /// - `source`: The full input source code.
    /// - `k`: The interval between entries in the coarse index.
    ///
    /// # Returns
    /// A new instance of `FastLineTable`.
    pub fn with_capacity(source: &str, k: usize) -> Self {
        const AVG_LINE_LENGTH: usize = 60;

        let mut line_starts = Vec::with_capacity(source.len() / AVG_LINE_LENGTH);
        line_starts.push(0);
        let mut coarse_index = vec![];

        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                let line_number = line_starts.len() + 1;
                line_starts.push(i + 1);

                if line_number % k == 0 {
                    coarse_index.push((i + 1, line_number));
                }
            }
        }

        Self {
            line_starts,
            coarse_index,
        }
    }

    /// Calculates the span (start and end positions) of a substring within the source text,
    /// including line and column information for both start and end positions.
    ///
    /// # Arguments
    ///
    /// * `start` - The byte index in the source string where the span starts.
    /// * `end` - The byte index in the source string where the span ends.
    /// * `source` - The entire source string from which the span is derived.
    ///
    /// # Returns
    ///
    /// Returns a `Span` struct containing the start and end byte indices along with
    /// corresponding line and column numbers within the source.
    ///
    /// # Notes
    ///
    /// This function iterates over the source string character by character,
    /// updating line and column counts, and stops once the end index is reached.
    /// It also handles the edge case where `end` equals the length of the source.
    fn get_span(&self, start: usize, end: usize) -> Span {
        let (sl, sc) = self.get_position(start);
        let (el, ec) = self.get_position(end);

        let mut span = Span::new(start, end);
        span.set_begin_line(sl);
        span.set_begin_column(sc);
        span.set_end_line(el);
        span.set_end_column(ec);

        span
    }


    /// Retrieves the line and column number corresponding to a given byte offset.
    ///
    /// # Arguments
    /// - `offset`: The byte offset in the source string.
    ///
    /// # Returns
    /// A tuple `(line_number, column_number)`, where:
    /// - `line_number` is the 1-based index of the line.
    /// - `column_number` is the 1-based index of the column within the line.
    fn get_position(&self, offset: usize) -> (usize, usize) {
        // Clip offset to maximum valid position (end of source)
        let max_offset = self.line_starts.last().copied().unwrap_or(0);
        let offset = offset.min(max_offset);

        // Fast lookup using the coarse index (binary search)
        let mut approx_line = match self
            .coarse_index
            .binary_search_by_key(&offset, |&(pos, _)| pos)
        {
            Ok(idx) => self.coarse_index[idx].1, // Exact match found
            Err(idx) => {
                if idx == 0 {
                    1 // If the offset is before the first indexed entry, start from line 1
                } else {
                    self.coarse_index[idx - 1].1 // Start from the nearest coarse index entry
                }
            }
        };

        // Fine-tune the search with a linear scan from the approximate starting point
        while approx_line < self.line_starts.len() && self.line_starts[approx_line] <= offset {
            approx_line += 1;
        }

        // Compute the column by subtracting the line start offset from the given offset
        let line_start = self.line_starts[approx_line - 1];
        (approx_line, offset - line_start + 1)
    }
}
