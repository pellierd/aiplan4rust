use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::DiagnosticSeverity;
use crate::aiplan4rust::diagnostic::DiagnosticSource;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::lexer::token::Token;
use crate::aiplan4rust::parser::lexer::Lexer;
use crate::aiplan4rust::parser::lexer::LexicalError;
use crate::aiplan4rust::parser::parser_result::ParserResult;
use crate::aiplan4rust::parser::pddl::HDDLParser;
use crate::aiplan4rust::parser::pddl::PDDLParser;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNode;
use crate::aiplan4rust::parser::syntax_tree::SyntaxTree;
use crate::aiplan4rust::parser::{Language, Span};

use lalrpop_util::ErrorRecovery;
use lalrpop_util::ParseError;

use std::mem;
use std::time::SystemTime;

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
    filename: Option<&'a str>,
    source: Option<&'a str>,
    diagnostic_manager: DiagnosticManager,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance.
    ///
    /// # Returns
    /// Returns a new instance of `Parser`.
    pub fn new() -> Self {
        Self {
            filename: None,
            source: None,
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
        filename: &'a str,
        source: &'a str,
        language: &Language,
    ) -> Result<ParserResult, ParserInternalError> {
        // Store temporary references to the filename and source for later use
        self.filename = Some(filename);
        self.source = Some(source);

        // Initialize a vector to store LALRPOP errors that may occur during parsing
        let mut larlpop_errors = Vec::new();

        // Create a lexer from the provided source code
        let lexer = Lexer::new(source);

        self.diagnostic_manager.add_source(filename.to_string(), source.to_string());

        // Attempt to parse the source code according to the language specified
        let parse_result = match language {
            Language::PDDL => PDDLParser::new().parse(&mut larlpop_errors, lexer),
            Language::HDDL => HDDLParser::new().parse(&mut larlpop_errors, lexer),
        };

        // Handle any syntax errors that were collected during parsing
        self.handle_syntax_errors(&larlpop_errors, source);

        if self
            .diagnostic_manager()
            .has_diagnotics_of_severity(DiagnosticSeverity::Error)
        {
            Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
        } else {
            match parse_result {
                Ok(mut ast) => {
                    self.process_ast(&mut ast, source)?;

                    //println!("AST: {}", ast);
                    if self
                        .diagnostic_manager()
                        .has_diagnotics_of_severity(DiagnosticSeverity::Error)
                    {
                        Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                    } else {
                        let syntax_tree =
                            SyntaxTree::new(ast, Some(filename.to_string()), SystemTime::now());
                        Ok(ParserResult::new(
                            Some(syntax_tree),
                            mem::take(&mut self.diagnostic_manager),
                        ))
                    }
                }
                Err(e) => {
                    let error = self.to_parser_error(
                        &e,
                        source,
                        Some(filename),
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
            let parser_error = self.to_parser_error(&larlpop_error.error, source, self.filename);
            self.diagnostic_manager.add_diagnostic(parser_error);
        }
    }

    /// Processes the abstract syntax tree (AST) based on the parsing result.
    /// This function normalizes the AST (keeps it in a Box) and initializes its position in the
    /// source code. If parsing fails or there are syntax errors, it returns `None`.
    ///
    /// # Arguments
    /// * `parse_result`: The result of the parsing attempt, containing the AST or an error.
    /// * `larlpop_errors`: A list of errors encountered during parsing.
    /// * `source`: The source code to initialize AST positions.
    fn process_ast(
        &mut self,
        ast: &mut Box<SyntaxNode>,
        source: &'a str,
    ) -> Result<(), ParserInternalError> {
        // Normalize the AST to ensure its structure is consistent
        //self.normalize_ast(ast)?;
        // Initialize the position of the AST elements in the source code
        self.init_ast_position(ast, source);
        Ok(())
    }

    /// Initializes the position information of an abstract syntax tree (AST).
    /// This function computes the line and column numbers for each node in the AST
    /// using a `FastLineTable`, which maps byte offsets to positions efficiently.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to the root node of the AST.
    /// - `source`: The source code string from which the AST was parsed.
    fn init_ast_position(&self, ast: &mut SyntaxNode, source: &str) {
        // Create a `FastLineTable` with an interval of 100 lines for coarse indexing.
        // The interval value (100) can be adjusted depending on the size of the source text.
        let table = FastLineTable::new(source, 100);

        // Recursively set positions for all AST nodes
        self.init_ast_position_rec(ast, &table, &mut 0);
    }

    /// Recursively sets the start and end positions (line, column) for each AST node.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to an AST node.
    /// - `table`: A reference to the `FastLineTable` used to compute positions.
    fn init_ast_position_rec(&self, ast: &mut SyntaxNode, table: &FastLineTable, id: &mut usize,) {
        // Assign an unique id to each node
        ast.set_id(*id);
        *id += 1;

        // Compute and set the start position of the current AST node
        let (line, column) = table.get_position(ast.start_offset());
        ast.set_start_position(line, column);

        // Compute and set the end position of the current AST node
        let (line, column) = table.get_position(ast.end_offset());
        ast.set_end_position(line, column);

        // Recursively process all child nodes of the current AST node
        for child in ast.children_mut() {
            self.init_ast_position_rec(child, table, id);
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

    fn get_span(&self, start: &usize, end: &usize, source: &str) -> Span {
        let mut line = 1;
        let mut column = 1;
        let mut sl = 1;
        let mut sc = 1;
        let mut el = 1;
        let mut ec = 1;

        for (i, ch) in source.char_indices() {
            if i == *start {
                sl = line;
                sc = column;
            }
            if i == *end {
                el = line;
                ec = column;
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        // Si end est égal à la longueur de la source, on doit capturer la dernière position manuellement
        if *end == source.len() {
            el = line;
            ec = column;
        }

        let mut span = Span::new(*start, *end);
        span.set_begin_line(sl);
        span.set_begin_column(sc);
        span.set_end_line(el);
        span.set_end_column(ec);
        span
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
                        expected:  clean_expected} ,
                    DiagnosticSource::Lexer,
                    file_path,
                    self.get_span(start, end, source),
                )
            }
            ParseError::InvalidToken { location } => {
                Diagnostic::new(
                    DiagnosticKind::InvalidToken,
                    DiagnosticSource::Lexer,
                    file_path,
                    self.get_span(location, location, source),
                )
            }
            ParseError::User { error } => {
                let content = error.to_string();
                Diagnostic::new(
                    DiagnosticKind::CustomError(content),
                    DiagnosticSource::Lexer,
                    file_path,
                    self.get_span(&0, &0, source),
                )
            }
            ParseError::UnrecognizedEof { location, expected } => {
                let clean_expected= Self::clean_expected(expected);
                Diagnostic::new(
                    DiagnosticKind::UnexpectedEof {
                        expected:  clean_expected} ,
                    DiagnosticSource::Lexer,
                    file_path,
                    self.get_span(location, location, source),
                )
            }
            ParseError::ExtraToken {
                token: (start, t, end),
            } => {
                Diagnostic::new(
                    DiagnosticKind::ExtraToken {
                        token:  t.to_string(),
                    },
                    DiagnosticSource::Lexer,
                    file_path,
                    self.get_span(start, end, source),
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
struct FastLineTable {
    line_starts: Vec<usize>,
    coarse_index: Vec<(usize, usize)>, // (Offset, Line number) every K lines
}

impl FastLineTable {
    /// Constructs a new `FastLineTable` from a given source string.
    ///
    /// # Arguments
    /// - `source`: The input string whose line positions will be indexed.
    /// - `k`: The interval at which the coarse index stores line offsets.
    ///
    /// # Returns
    /// A new instance of `FastLineTable`.
    fn new(source: &str, k: usize) -> Self {
        let mut line_starts = vec![0]; // The first line always starts at offset 0
        let mut coarse_index = vec![];

        // Iterate through each byte in the source string
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                let line_number = line_starts.len() + 1; // Compute the next line number
                line_starts.push(i + 1); // Store the offset of the next line

                // Store coarse index entry every `k` lines
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
