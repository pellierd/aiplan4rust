use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Severity;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::lexer::Lexer;
use crate::aiplan4rust::syntax::lexer::LexicalError;
use crate::aiplan4rust::syntax::{ArenaParserResult, FastLineTable, Language, ParseContext, ParserError};
use crate::aiplan4rust::syntax::ast::AstArena;
use crate::aiplan4rust::syntax::lalrpop;

use lalrpop_util::ErrorRecovery;

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
    diagnostic_manager: DiagnosticManager,
}

impl<'a> Parser<'a> {
    /// Creates a new `Parser` instance with empty source and diagnostics.
    ///
    /// Initializes the parser with no source code loaded and a fresh diagnostic manager.
    ///
    /// # Returns
    /// A new instance of `Parser` ready to parse source code.
    pub fn new() -> Self {
        Self {
            source_name: None,
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
        source_name: &'a str,
        source: &'a str,
        language: &Language,
    ) -> Result<ArenaParserResult, ParserInternalError> {

        // 1. Store temporary references to the filename and source code
        //    These will be used later for generating diagnostics with context.
        self.source_name = Some(source_name);
        self.source = Some(source);
        self.diagnostic_manager.add_source(source_name.to_string(), source.to_string());

        // 2. Initialize a vector to collect any error recovery issues from LALRPOP.
        //    You plan to collect these errors during parsing.
        let mut errors = Vec::new();

        // 3. Create a lexer from the source code to tokenize the input.
        let lexer = Lexer::new(source);

        // 4. Create a parsing context which holds parser state and memory during parsing.
        let mut context = ParseContext::new();

        // 5. Run the parser depending on the specified language (PDDL or HDDL).
        //    The parser returns either Ok(root_id) representing the AST root,
        //    or an error.
        let parse_result = match language {
            Language::PDDL => lalrpop::parse_pddl(&mut context, lexer),
            Language::HDDL => lalrpop::parse_hddl(&mut context, lexer),
        };

        // 6. Extract the string interner from the context.
        //    It manages deduplicated string storage, etc.
        let interner = context.take_interner();

        // 7. Build a fast line table for quick line and column lookups.
        let fast_line_table = FastLineTable::new(source);

        // 8. Handle any syntax errors collected and convert them into diagnostics.
        self.handle_syntax_diagnostics(&errors, &fast_line_table);

        // 9. If any diagnostics of severity Error exist, return early with no AST.
        if self.diagnostic_manager().has_diagnotics_of_severity(Severity::Error) {
            return Ok(ArenaParserResult::new(None, mem::take(&mut self.diagnostic_manager)));
        }

        // 10. Analyze the parser result.
        match parse_result {
            // Success: we have a root AST node.
            Ok(root_id) => {
                // Check again for errors in diagnostics after parsing.
                if self.diagnostic_manager().has_diagnotics_of_severity(Severity::Error) {
                    Ok(ArenaParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                } else {
                    // Otherwise, take the arena and create the AST.
                    let mut arena = context.take_arena();
                    let mut ast = AstArena::new(
                        arena,
                        interner,
                        source_name.to_string(),
                        SystemTime::now(),
                    );
                    // Initialize line and column spans on each AST node.
                    ast.init_span(&fast_line_table)?;
                    // Return the AST with the diagnostics.
                    Ok(ArenaParserResult::new(Some(ast), mem::take(&mut self.diagnostic_manager)))
                }
            }
            // Parsing error occurred.
            Err(e) => match e {
                // Normal parse error: convert it to a diagnostic.
                ParserError::ParseError(err) => {
                    let diagnostic = Diagnostic::from_parse_error(&err, Some(source_name), &fast_line_table);
                    self.diagnostic_manager.add_diagnostic(diagnostic);
                    // Return no AST but updated diagnostics.
                    Ok(ArenaParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                }
                // Internal error: escalate it as a fatal error.
                ParserError::InternalError(err) => {
                    Err(err.into())
                }
            },
        }
    }


    /// Handles syntax errors produced by the LALRPOP parser by converting them into diagnostics.
    ///
    /// For each LALRPOP error, this function creates a corresponding `Diagnostic` and adds it
    /// to the diagnostic manager for reporting.
    ///
    /// # Arguments
    /// * `lalrpop_errors` - Slice of errors produced by the LALRPOP parser.
    /// * `fast_line_table` - A reference to a `FastLineTable` used to map byte positions to line/column.
    ///
    /// # Behavior
    /// This function iterates over all parsing errors, converts each to a diagnostic message
    /// with source location info, and records them in the diagnostic manager.
    fn handle_syntax_diagnostics(
        &mut self,
        lalrpop_errors: &[ErrorRecovery<usize, Token, LexicalError>],
        fast_line_table: &FastLineTable,
    ) {
        for error_recovery in lalrpop_errors {
            let diagnostic = Diagnostic::from_parse_error(&error_recovery.error, self.source_name, fast_line_table);
            self.diagnostic_manager.add_diagnostic(diagnostic);
        }
    }

}
