use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::syntax::lexer::LexicalError;
use crate::aiplan4rust::syntax::parser_result::ParserResult;
use crate::aiplan4rust::syntax::grammar::HDDLParser;
use crate::aiplan4rust::syntax::grammar::PDDLParser;
use crate::aiplan4rust::syntax::{FastLineTable, Language, ParseContext};
use crate::aiplan4rust::syntax::ast::{Ast, AstNode};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::tree::TreeArena;

use lalrpop_util::ErrorRecovery;
use lalrpop_util::ParseError;

use std::mem;
use std::time::SystemTime;


#[derive(Debug)]
/// A structure for analyzing the syntax of PDDL expr.
///
/// The `SyntaxAnalyzer` is responsible for parsing and validating PDDL expr from
/// a given source string. It works with an optional path for file-based sources and an
/// error manager to handle any parsing errors.
///
/// # Fields
/// - `source`: A reference to the source string containing the PDDL expr to analyze.
/// - `path`: An optional `PathBuf` representing the file path from which the source is read.
/// - `pddl_fragment`: The `PDDLFragment` that represents the parsed expr.
/// - `error_manager`: A mutable reference to the `ErrorManager` for managing parsing errors.
///
/// # Example
/// ```rust
/// use aiplan4rust::aiplan4rust::syntax::syntax::PDDLFragment;
/// let source = "(define (problem test) ...)";
/// let mut error_manager = ErrorManager::new();
/// let pddl_expression = PDDLFragment::Domain; // Example expr
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
        self.diagnostic_manager.add_source(source_name.to_string(), source.to_string());

        // Initialize a vector to store LALRPOP errors that may occur during parsing
        let mut errors = Vec::new();
        // Create a lexer from the provided source code
        let lexer = Lexer::new(source);

//        let mut interner = StringInterner::new();
//        let mut ast = TreeArena::<AstArenaNode>::new();
        let mut context = ParseContext::new();

        // Attempt to parse the source code according to the language specified
        let parse_result = match language {
            Language::PDDL => PDDLParser::new().parse(&context, lexer),
            Language::HDDL => HDDLParser::new().parse(&context, lexer),
        };

        let interner = context.take_interner();
        // Create a `FastLineTable` with an interval for coarse indexing.
        self.fast_line_table = FastLineTable::new(source);

        // Handle any syntax errors that were collected during parsing
        self.handle_syntax_errors(&errors);

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
                            Ast::new(root, interner, source_name.to_string(), SystemTime::now());
                        Ok(ParserResult::new(
                            Some(ast),
                            mem::take(&mut self.diagnostic_manager),
                        ))
                    }
                }
                Err(e) => {
                    let error = self.to_parser_error(
                        &e,
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
    ) {
        for larlpop_error in larlpop_errors {
            // Convert each LALRPOP error into a ParserError and add it to the error manager
            let parser_error = self.to_parser_error(&larlpop_error.error, self.source_name);
            self.diagnostic_manager.add_diagnostic(parser_error);
        }
    }

    /// Recursively sets the start and end positions (line, column) for each AST node.
    ///
    /// # Arguments
    /// - `ast_old`: A mutable reference to an AST node.
    fn init_ast_span(&self, ast: &mut AstNode) {
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
