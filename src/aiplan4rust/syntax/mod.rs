/// The `aiplan4rust` module provides core components for parsing PDDL and HDDL files,
/// including parsing logic, error handling, and diagnostics rendering.
///
/// # Overview
/// This module enables reading, parsing, and error reporting for planning domain
/// description languages (PDDL and HDDL). It manages syntax analysis and presents
/// clear diagnostics to users.
///
/// # Example
/// The following example demonstrates how to read a file, parse its content with the `Parser`,
/// and render any parsing errors encountered.
///
/// ```rust
/// use aiplan4rust::{Parser, Renderer};
/// use std::fs::File;
/// use std::io::Read;
///
/// /// Reads the entire content of the file at `path` into a String.
/// /// Returns an I/O error if the file cannot be read.
/// fn read_file(path: &str) -> Result<String, std::io::Error> {
///     let mut file = File::open(path)?;
///     let mut content = String::new();
///     file.read_to_string(&mut content)?;
///     Ok(content)
/// }
///
/// let source_path = "domain.pddl";  // Specify your domain file path here.
///
/// let content = read_file(source_path)?;
///
/// let mut parser = Parser::new();
///
/// // Replace `language` with the appropriate Language enum variant.
/// let parser_result = parser.parse(source_path, &content, language)?;
///
/// let mut renderer = Renderer::new(parser_result.diagnostic_manager());
/// renderer.display();
/// ```
///
/// # Explanation
/// This module encapsulates the workflow of reading PDDL/HDDL source files,
/// parsing them, and rendering diagnostics for any errors encountered.
/// Errors are managed via the `DiagnosticRenderer` to provide clear and
/// user-friendly error messages.
///
/// # Notes
/// - Focus is on syntax parsing and error reporting.
/// - You must specify the `language` parameter to indicate the parsing language (PDDL/HDDL).
///
/// # Example Output
/// If parsing errors occur, detailed error messages with line and column information
/// will be displayed to help locate and fix issues.
///
/// # Syntax Display Trait
/// The `SyntaxDisplay` trait provides a standardized interface for converting
/// AST nodes into their syntax string representations. This abstraction supports
/// multiple planning domain languages such as PDDL and HDDL, enabling consistent
/// and customizable pretty-printing and serialization of AST structures.
///
/// Implementors of `SyntaxDisplay` must provide methods to produce a string
/// representation optionally respecting indentation or depth for formatting.
///
/// # Modules
/// This crate exposes submodules for language definitions, lexical analysis,
/// parsing, grammar rules, AST structures, spans, parser results, and syntax display.
///
/// # Exports
/// Key components like `Language`, `Parser`, `ParserResult`, `Span`, `AstKind`, `AstNode`,
/// and `SyntaxDisplay` are re-exported for convenient external use.
pub mod elements;
pub mod language;
pub mod lexer;
pub mod parser;
pub mod parser_result;
pub mod grammar;
pub mod span;
pub mod ast;
pub mod display;

pub mod string_interner;

pub use language::Language;
pub use parser::Parser;
pub use parser_result::ParserResult;
pub use span::Span;
pub use ast::AstKind;
pub use ast::AstNode;
pub use display::Display as SyntaxDisplay;
pub use string_interner::StringInterner;
pub mod int_ast;
