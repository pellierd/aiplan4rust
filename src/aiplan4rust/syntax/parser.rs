//! Parser module responsible for analyzing PDDL/HDDL syntax and producing ASTs along with diagnostics.
//!
//! This module provides the `Parser` struct, which handles lexing, parsing, and diagnostics management
//! for PDDL or HDDL source code. It integrates with the underlying lexer, parser (via LALRPOP), and
//! diagnostic system to provide detailed parsing results and error reporting.

use lalrpop_util::ErrorRecovery;
use std::mem;
use std::time::SystemTime;
use logos::Logos;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Severity};
use crate::aiplan4rust::interner::Literal;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::lalrpop;
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::lexer::{Lexer, LexicalError};
use crate::aiplan4rust::syntax::{
    FastLineTable, Language, ParseContext, ParserResult, SyntaxError,
};

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
    source_name: &'a str,
    source_content: &'a str,
    diagnostic_manager: DiagnosticManager,
}

impl<'a> Parser<'a> {
    /// Creates a new, empty `Parser` instance.
    ///
    /// Initializes the parser with empty diagnostic manager ready to collect diagnostics.
    ///
    /// This means the parser starts with no loaded input and is ready to
    /// receive source code for parsing.
    ///
    /// # Returns
    ///
    /// A `Parser` instance with empty source fields and an initialized diagnostic manager.
    pub fn new() -> Self {
        Self {
            source_name: "",
            source_content: "",
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
    /// * `source_content` - The source code to parse.
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
        source_content: &'a str,
    ) -> Result<ParserResult, SyntaxError> {
        // Store the source name (e.g., filename) for diagnostics context
        self.source_name = source_name;
        // Store the source code string slice for diagnostics context
        self.source_content = source_content;

        // Create a new lexer instance from the source text to tokenize input
        let lexer = Lexer::new(source_content);
        // Initialize the parser context which holds parser state and memory allocations
        let mut context = ParseContext::new();

        // Run the parser for the specified language variant (PDDL or HDDL)
        // Returns Ok(root_id) on success or Err(ParserError) on failure
        let language = Parser::detect_language(source_content);
        let parse_result = match language {
            Language::PDDL => lalrpop::parse_pddl(&mut context, lexer),
            Language::HDDL => lalrpop::parse_hddl(&mut context, lexer),
        };

        // Extract the string interner from the parsing context (used to store unique strings)
        let mut interner = context.take_interner();
        let source_id = interner.intern_literal(source_name);

        // Register the source text with the diagnostic manager
        self.diagnostic_manager
            .add_source(source_id, source_content.to_string());

        // Build a fast line table from the source for quick byte-to-line/column lookups
        let fast_line_table = FastLineTable::new(source_content);

        // Convert any collected LALRPOP errors into diagnostics and add them to the manager
        self.handle_syntax_diagnostics(&context.borrow_errors_mut(), source_id, &fast_line_table);

        // If any error-level diagnostics were added, parsing failed—return no AST but diagnostics
        if self
            .diagnostic_manager()
            .has_diagnostics_of_severity(Severity::Error)
        {
            return Ok(ParserResult::failure(
                mem::take(&mut self.diagnostic_manager),
                interner,
            ));
        }

        // Process the result of the parsing operation
        match parse_result {
            Ok(_) => {
                // Double-check if any errors were added during parsing
                if self
                    .diagnostic_manager()
                    .has_diagnostics_of_severity(Severity::Error)
                {
                    // Return failure with diagnostics if errors are present
                    Ok(ParserResult::failure(
                        mem::take(&mut self.diagnostic_manager),
                        interner,
                    ))
                } else {
                    // Take ownership of the arena holding parsed nodes
                    let arena = context.take_syntax_tree();
                    // Create an AST instance from the arena, interner, source name, and timestamp
                    let mut ast =
                        Ast::new(arena, interner, source_id, SystemTime::now());
                    // Initialize line/column span info for AST nodes using the line table
                    ast.init_span(&fast_line_table)?;

                    // Return the successful parse result with AST and diagnostics
                    Ok(ParserResult::success(
                        ast,
                        mem::take(&mut self.diagnostic_manager),
                    ))
                }
            }
            Err(e) => match e.as_parse_error() {
                Some(parse_err) => {
                    let source = interner.intern_literal(source_name);
                    let diagnostic =
                        Diagnostic::from((parse_err, source, &fast_line_table));
                    self.diagnostic_manager.add_diagnostic(diagnostic);
                    Ok(ParserResult::failure(
                        mem::take(&mut self.diagnostic_manager),
                        interner,
                    ))
                }
                None => Err(e),
            },
        }
    }


    /// Detects whether an input string represents a PDDL or HDDL file.
    ///
    /// This function performs a lightweight lexical scan over the input text using the
    /// existing `logos`-based lexer (`Token::lexer`). The goal is to identify language-specific
    /// tokens that appear **exclusively in HDDL** and never in valid PDDL input.
    ///
    /// ## Detection Strategy
    ///
    /// HDDL introduces several constructs that do not exist in PDDL:
    ///
    /// - `:task`
    /// - `:method`
    /// - `:method-preconditions`
    /// - `:hierarchy`
    /// - `:htn` (in problem files)
    ///
    /// During lexing, if any of these tokens are encountered, the function can conclude
    /// **with certainty** that the input is HDDL.
    ///
    /// If none of the HDDL-exclusive tokens are found, the input is assumed to be PDDL.
    ///
    /// ## Performance
    ///
    /// - The function performs **a single pass** of the Logos lexer over the input.
    /// - No parsing is performed.
    /// - This makes it extremely fast and suitable to run before calling the appropriate
    ///   LALRPOP parser (`parse_pddl` or `parse_hddl`).
    ///
    /// ## Returns
    ///
    /// - [`Language::HDDL`] if at least one HDDL-specific token is found.
    /// - [`Language::PDDL`] otherwise.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// let src = "(:method m1 ...)";
    /// assert_eq!(detect_language(src), Language::HDDL);
    ///
    /// let src = "(:action move ...)";
    /// assert_eq!(detect_language(src), Language::PDDL);
    /// ```
    ///
    /// ## Guarantees
    ///
    /// This detection is **sound and unambiguous**:
    /// no valid PDDL file contains HDDL-specific tokens.
    ///
    /// # Parameters
    /// - `input`: The raw text of the domain or problem file.
    ///
    /// # See Also
    /// - [`Token`] — the lexer token type generated by Logos.
    /// - [`Language`] — enumeration representing PDDL or HDDL.
    ///
    /// # Panics
    /// This function never panics.
    ///
    /// # Notes
    /// Lexical errors from Logos are ignored during detection, as they cannot help
    /// determine the language and may occur in partially written files.
    pub fn detect_language(input: &str) -> Language {

        let mut lexer = Token::lexer(input);

        while let Some(token) = lexer.next() {
            match token {
                Ok(tok) => match tok {
                    Token::Task
                    | Token::Method
                    | Token::MethodPreconditions
                    | Token::Hierarchy
                    | Token::Htn => {
                        return Language::HDDL;
                    }
                    _ => {}
                },
                Err(_) => {}
            }
        }

        Language::PDDL
    }

    /// Converts LALRPOP parsing errors into `Diagnostic` instances and registers them
    /// with the `DiagnosticManager` for centralized error tracking.
    ///
    /// This function processes all parser error recoveries (`ErrorRecovery`) produced
    /// during parsing, converting each into a structured diagnostic that includes
    /// span information and a reference to the source file via an interned identifier.
    ///
    /// # Arguments
    ///
    /// * `lalrpop_errors` - A slice of parser error recoveries emitted by LALRPOP.
    /// * `source_id` - An interned `Literal` identifying the source file in which the errors occurred.
    /// * `fast_line_table` - A line/column lookup structure used to compute error spans from byte positions.
    ///
    /// # Behavior
    ///
    /// - Iterates through each `ErrorRecovery`.
    /// - Converts each `ParseError` into a `Diagnostic`, using the source file identifier and line table.
    /// - Adds the generated diagnostic to the `DiagnosticManager` for later reporting.
    ///
    /// # Note
    ///
    /// - This function does not return anything, as diagnostics are registered directly with the manager.
    ///
    /// # Example
    ///
    /// ```ignore
    /// manager.handle_syntax_diagnostics(&errors, file_id, &line_table);
    /// ```
    fn handle_syntax_diagnostics(
        &mut self,
        lalrpop_errors: &[ErrorRecovery<usize, Token, LexicalError>],
        source_id: Literal,
        fast_line_table: &FastLineTable,
    ) {
        for error_recovery in lalrpop_errors {
            let diagnostic =
                Diagnostic::from((&error_recovery.error, source_id, fast_line_table));
            self.diagnostic_manager.add_diagnostic(diagnostic);
        }
    }


}
