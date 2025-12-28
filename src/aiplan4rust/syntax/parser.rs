//! Parser module responsible for analyzing PDDL/HDDL syntax and producing ASTs along with diagnostics.
//!
//! This module provides the `Parser` struct, which handles lexing, parsing, and diagnostics management
//! for PDDL or HDDL source code. It integrates with the underlying lexer, parser (via LALRPOP), and
//! diagnostic system to provide detailed parsing results and error reporting.

use lalrpop_util::ErrorRecovery;
use std::mem;
use std::time::SystemTime;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Severity};
use crate::aiplan4rust::interner::Literal;
use crate::aiplan4rust::artefact::source::Source;
use crate::aiplan4rust::artefact::language::Language;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::lalrpop;
use crate::aiplan4rust::syntax::lexer::token::Token;
use crate::aiplan4rust::syntax::lexer::{Lexer, LexicalError};
use crate::aiplan4rust::syntax::{
    FastLineTable, ParseContext, ParserResult, SyntaxError,
};

/// Parses PDDL or HDDL source code into an abstract syntax tree (AST),
/// while managing and reporting diagnostics (errors, warnings, notes).
///
/// The `Parser` struct maintains a diagnostic manager and tracks the
/// source content and source name internally. Parsing respects the
/// `SourceInfo` of the provided `Source`:
/// - `SourceInfo::Raw` — parsed according to the detected language (PDDL/HDDL)
/// - `SourceInfo::Serialized` — cannot be parsed, returns `SyntaxError::UnexpectedSerializedSource`
/// - `SourceInfo::Unknown` — cannot be parsed, returns `SyntaxError::UnknownSource`
///
/// # Example
/// ```rust
/// use aiplan4rust::aiplan4rust::artefact::source::Source;
/// use aiplan4rust::aiplan4rust::syntax::Parser;
///
/// let input = Source::read_from_file("./domain.pddl");
/// let mut parser = Parser::new();
/// let result = parser.parse(&input);
///
/// match result {
///     Ok(parser_result) => {
///         if parser_result.is_success() {
///             println!("Parsed successfully!");
///         } else {
///             println!("Parsing failed with diagnostics:");
///             for diag in parser_result.diagnostic_manager().diagnostics() {
///                 println!("{}", diag);
///             }
///         }
///     }
///     Err(err) => eprintln!("Parsing error: {:?}", err),
/// }
/// ```
#[derive(Debug)]
pub struct Parser {
    diagnostic_manager: DiagnosticManager,
}

impl Parser {
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

    /// Parses the given source, producing a parse result.
    ///
    /// The parser inspects the `Source` to determine whether it contains raw PDDL/HDDL content
    /// or a serialized source. If the source is raw, it selects the appropriate parser
    /// (PDDL or HDDL) based on the detected language. Serialized or unknown sources
    /// cannot be parsed directly and will return an error.
    ///
    /// # Arguments
    ///
    /// * `source` - A reference to the `Source` to parse.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Ok(ParserResult)` if parsing was successful or produced recoverable errors.
    /// - `Err(SyntaxError)` if an unrecoverable error occurred, e.g., the source is serialized or unknown.
    ///
    /// # Errors
    ///
    /// - `SyntaxError::UnexpectedSerializedSource` if the source is serialized.
    /// - `SyntaxError::UnknownSource` if the source format could not be determined.
    /// - Lexical and syntactic errors from parsing are captured in the `ParserResult` diagnostics.
    pub fn parse(
        &mut self,
        source: &Source,
    ) -> Result<ParserResult, SyntaxError> {

        // Run the parser for the specified language variant (PDDL or HDDL)
        let content = source.try_raw_content()?; // Get raw info, or return error if source is Serialized/Unknown
        let inner = content.inner();

        let mut context = ParseContext::new(); // Initialize a new parsing context
        let lexer = Lexer::new(inner); // Create a lexer for tokenizing the source content

        // Parse the source according to its detected language (PDDL or HDDL)
        let parse_result = match content.language() {
            Language::PDDL => lalrpop::parse_pddl(&mut context, lexer),
            Language::HDDL => lalrpop::parse_hddl(&mut context, lexer),
        };

        // Extract the string interner from the parsing context (used to store unique strings)
        let mut interner = context.take_interner();
        let source_id = interner.intern_literal(source.path().to_string_lossy());

        // Register the source text with the diagnostic manager
        self.diagnostic_manager
            .add_source(source_id, inner.to_string());

        // Build a fast line table from the source for quick byte-to-line/column lookups
        let fast_line_table = FastLineTable::new(inner);

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
                    let source = interner.intern_literal(source.path().to_string_lossy());
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
