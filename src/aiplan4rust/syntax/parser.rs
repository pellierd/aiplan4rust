//! Parser module responsible for analyzing PDDL/HDDL syntax and producing ASTs along with diagnostics.
//!
//! This module provides the `Parser` struct, which handles lexing, parsing, and diagnostics management
//! for PDDL or HDDL source code. It integrates with the underlying lexer, parser (via LALRPOP), and diagnostic system
//! to provide detailed parsing results and error reporting.

use lalrpop_util::ErrorRecovery;
use std::mem;
use std::time::SystemTime;

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Severity};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::lexer::{Lexer, LexicalError};
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::{FastLineTable, Language, ParseContext, ParserError, ParserResult};
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::lalrpop;
use crate::check_well_formed;

/// Parses PDDL or HDDL source code into an abstract syntax tree (AST),
/// while managing and reporting diagnostics (errors, warnings, notes).
///
/// The `Parser` struct maintains internal references to the source,
/// source name, and a diagnostic manager to collect and expose parsing issues.
///
/// # Example
/// ```rust
/// use aiplan4rust::aiplan4rust::syntax::{Parser, Language};
///
/// let source_code = "(define (problem example) ...)";
/// let mut parser = Parser::new();
/// let result = parser.parse("example.pddl", source_code, &Language::PDDL);
///
/// match result {
///     Ok(parser_result) => {
///         if parser_result.is_some() {
///             println!("Parsed successfully!");
///         } else {
///             println!("Parsing failed with diagnostics:");
///             for diag in parser_result.diagnostic_manager().diagnostics() {
///                 println!("{}", diag);
///             }
///         }
///     }
///     Err(internal_error) => eprintln!("Internal parser error: {:?}", internal_error),
/// }
/// ```
#[derive(Debug)]
pub struct Parser<'a> {
    source_name: Option<&'a str>,
    source: Option<&'a str>,
    diagnostic_manager: DiagnosticManager,
}

impl<'a> Parser<'a> {
    /// Creates a new, empty `Parser` instance.
    ///
    /// The parser starts with no loaded source and a fresh diagnostic manager.
    ///
    /// # Returns
    ///
    /// A new `Parser` ready to parse source code.
    pub fn new() -> Self {
        Self {
            source_name: None,
            source: None,
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the parser's diagnostic manager.
    ///
    /// The diagnostic manager contains all errors, warnings, and notes produced during parsing.
    ///
    /// # Returns
    ///
    /// An immutable reference to the `DiagnosticManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Parses the given source code string in the specified language, producing a parse result.
    ///
    /// # Arguments
    ///
    /// * `source_name` - A string slice identifying the source (e.g., filename) for diagnostics.
    /// * `source` - The source code to parse.
    /// * `language` - The language variant (`PDDL` or `HDDL`) to use for parsing.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Ok(ParserResult)` if parsing was successful or produced recoverable errors.
    /// - `Err(ParserInternalError)` if an unrecoverable internal error occurred.
    ///
    /// # Errors
    ///
    /// Lexical and syntactic errors are captured as diagnostics within the `ParserResult`.
    /// Internal errors cause this function to return an `Err`.
    pub fn parse(
        &mut self,
        source_name: &'a str,
        source: &'a str,
        language: &Language,
    ) -> Result<ParserResult, ParserInternalError> {
        // Store the source name (e.g., filename) for diagnostics context
        self.source_name = Some(source_name);
        // Store the source code string slice for diagnostics context
        self.source = Some(source);
        // Register the source text with the diagnostic manager
        self.diagnostic_manager.add_source(source_name.to_string(), source.to_string());

        // Create a new lexer instance from the source text to tokenize input
        let lexer = Lexer::new(source);
        // Initialize the parser context which holds parser state and memory allocations
        let mut context = ParseContext::new();

        // Run the parser for the specified language variant (PDDL or HDDL)
        // Returns Ok(root_id) on success or Err(ParserError) on failure
        let parse_result = match language {
            Language::PDDL => lalrpop::parse_pddl(&mut context, lexer),
            Language::HDDL => lalrpop::parse_hddl(&mut context, lexer),
        };

        // Extract the string interner from the parsing context (used to store unique strings)
        let interner = context.take_interner();
        // Build a fast line table from the source for quick byte-to-line/column lookups
        let fast_line_table = FastLineTable::new(source);

        // Convert any collected LALRPOP errors into diagnostics and add them to the manager
        self.handle_syntax_diagnostics(&context.borrow_errors_mut(), &fast_line_table);

        // If any error-level diagnostics were added, parsing failed—return no AST but diagnostics
        if self.diagnostic_manager().has_diagnotics_of_severity(Severity::Error) {
            return Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)));
        }

        // Process the result of the parsing operation
        match parse_result {
            Ok(_) => {
                // Double-check if any errors were added during parsing
                if self.diagnostic_manager().has_diagnotics_of_severity(Severity::Error) {
                    // Return failure with diagnostics if errors are present
                    Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                } else {
                    // Take ownership of the arena holding parsed nodes
                    let arena = context.take_arena();
                    // Create an AST instance from the arena, interner, source name, and timestamp
                    let mut ast = Ast::new(
                        arena,
                        interner,
                        source_name.to_string(),
                        SystemTime::now(),
                    );
                    // Initialize line/column span info for AST nodes using the line table
                    ast.init_span(&fast_line_table)?;

                    println!("{}", ast.to_string_with_interner());
                    match check_well_formed(&ast) {
                        Ok(()) => {
                            println!("Validation successful: no errors.");
                        }
                        Err(e) => {
                            println!("Validation failed:\n{}", e);

                        }
                    }

                    // Return the successful parse result with AST and diagnostics
                    Ok(ParserResult::new(Some(ast), mem::take(&mut self.diagnostic_manager)))
                }
            }
            Err(e) => match e {
                // Handle normal parse errors by converting them to diagnostics
                ParserError::ParseError(err) => {
                    let diagnostic =
                        Diagnostic::from_parse_error(&err, Some(source_name), &fast_line_table);
                    self.diagnostic_manager.add_diagnostic(diagnostic);
                    // Return a ParserResult with no AST but populated diagnostics
                    Ok(ParserResult::new(None, mem::take(&mut self.diagnostic_manager)))
                }
                // Propagate internal errors as fatal parsing failures
                ParserError::InternalError(err) => Err(err.into()),
            },
        }
    }


    /// Converts LALRPOP parsing errors into diagnostics and adds them to the diagnostic manager.
    ///
    /// # Arguments
    ///
    /// * `lalrpop_errors` - Slice of LALRPOP error recoveries collected during parsing.
    /// * `fast_line_table` - Fast line and column lookup table for mapping error positions.
    ///
    /// # Behavior
    ///
    /// Iterates through each parsing error and generates a corresponding diagnostic
    /// for accurate and user-friendly error reporting.
    fn handle_syntax_diagnostics(
        &mut self,
        lalrpop_errors: &[ErrorRecovery<usize, Token, LexicalError>],
        fast_line_table: &FastLineTable,
    ) {
        for error_recovery in lalrpop_errors {
            let diagnostic =
                Diagnostic::from_parse_error(&error_recovery.error, self.source_name, fast_line_table);
            self.diagnostic_manager.add_diagnostic(diagnostic);
        }
    }
}
