/// The `aiplan4rust` module provides the necessary components for parsing PDDL and HDDL files.
/// It includes tools for parsing, error management, and diagnostic display.
///
/// # Example
/// This example demonstrates how to read a file, parse its content using the `Parser`,
/// and display any parsing errors encountered.
///
/// ```rust
/// use aiplan4rust::{Parser, DiagnosticRenderer};
/// use std::fs::File;
/// use std::io::Read;
///
/// // This function reads the content of a file located at the specified path and returns
/// // it as a `String`. If an error occurs during the file reading process, an error is returned.
/// fn read_file(path: &str) -> Result<String, std::io::Error> {
///     let mut file = File::open(path)?;
///     let mut content = String::new();
///     file.read_to_string(&mut content)?;
///     Ok(content)
/// }
///
/// // Path to the source file.
/// let source_path = "domain.pddl";  // Ensure correct file path string format.
///
/// // Attempt to read the content of the source file.
/// let content = read_file(source_path)?;
///
/// // Create a new parser instance.
/// let mut parser = Parser::new();
///
/// // Attempt to parse the content, returning the result in `parser_result`.
/// let mut parser_result = parser.parse(source_path, &content, language)?;
///
/// // Display the errors from the parser if any exist.
/// let mut renderer = DiagnosticRenderer::new(parser_result.diagnostic_manager());
///renderer.display();
/// ```
///
/// # Explanation
/// This module provides an interface to read a PDDL or HDDL file, parse it, and display
/// any parsing errors. Errors are managed by the `DiagnosticRenderer`, which makes the
/// diagnostics easy to understand and visually accessible.
///
/// # Notes
/// - This example focuses on the workflow of reading a file, parsing it, and displaying errors.
/// - The `DiagnosticRenderer` is used to print the errors found during the parsing process.
///
/// # Example Output
/// In case of errors, the output will display a detailed message for each error found
/// during the parsing process.
pub mod elements;
pub mod language;
pub mod lexer;
pub mod parser;
pub mod parser_result;
pub mod pddl;
pub mod symbol_origin;
pub mod span;
pub mod syntax_tree;

/// Export key components for external usage of the module.
/// This allows access to types like `Language`, `Parser`, `ParserResult`,
/// `SymbolOrigin`, and `Span` directly.
pub use language::Language;
pub use parser::Parser;
pub use parser_result::ParserResult;
pub use symbol_origin::SymbolOrigin;
pub use span::Span;
