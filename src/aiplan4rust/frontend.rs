use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::linking::Linker;
use crate::aiplan4rust::linking::LinkerResult;
use crate::aiplan4rust::syntax::Parser;
use crate::aiplan4rust::semantic::{Analyzer, SemanticContext};
use crate::aiplan4rust::normalization::Normalizer;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::lir::{LirBuilder, LirBuilderResult};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;

use std::fs::File;
use std::io::Read;
use std::string::String;


#[derive(Debug)]
pub struct Frontend {}

impl Frontend {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse the domain and problem files, performing a semantic linking process.
    ///
    /// This function parses the domain and problem files, then attempts to link them together.
    /// If the annotated syntax trees of the domain or problem are `None`, it returns an error.
    /// If any issues occur during the linking process, they will be captured and displayed.
    ///
    /// # Arguments
    /// - `domain_path`: Path to the domain file.
    /// - `problem_path`: Path to the problem file.
    /// - `language`: The language to specify whether to parse as PDDL or HDDL.
    ///
    /// # Returns
    /// - `Ok(())` if the linking was successful.
    /// - `Err(ParserInternalError)` if any errors occurred during parsing or linking.
    ///
    /// # Errors
    /// - If the syntax arena of the domain or problem is `None`, an error is returned.
    /// - If the linking process fails, the error is returned.
    /// Parses, analyzes, links, and builds an intermediate representation (LIR)
    /// from a domain and a problem file.
    ///
    /// # Arguments
    ///
    /// * `domain_path` - Path to the domain file.
    /// * `problem_path` - Path to the problem file.
    ///
    /// # Returns
    ///
    /// * `Ok(LirBuilderResult)` on success, containing the LIR or diagnostics.
    /// * `Err(AiplanError)` on failure during parsing, normalization, or analysis.
    pub fn parse(
        &self,
        domain_path: &str,
        problem_path: &str,
    ) -> Result<LirBuilderResult, AiplanError> {
        // Step 1: Parse, normalize, and analyze both domain and problem files
        let domain = self.parse_file(domain_path)?;
        let problem = self.parse_file(problem_path)?;

        // Step 2: Link domain and problem semantic contexts
        let mut linker = Linker::new();
        let mut linker_result = linker.link(domain, problem)?;

        // Step 3: If linking succeeded, build the LIR
        if let Some(mut linked_semantic_context) = linker_result.take_linked_semantic_context() {
            let mut ir_builder = LirBuilder::new();
            let builder_result = ir_builder.build_with_diagnostic_manager(
                &mut linked_semantic_context,
                linker_result.take_diagnostic_manager(),
            )?;
            Ok(builder_result)
        } else {
            //println!("Linking failed, no linked semantic context available.");
            // Linking failed: return diagnostics and interner only
            let diagnostic_manager = linker_result.take_diagnostic_manager();
            let interner = linker_result.take_interner();
            Ok(LirBuilderResult::failure(diagnostic_manager, interner))
        }
    }

    /// Parses the source file at the given path and performs semantic analysis on the parsed syntax
    /// arena.
    ///
    /// This function performs the following steps:
    /// 1. Reads the content of the source file specified by `source_path`.
    /// 2. Uses the `Parser` to parse the content into a `SyntaxTree`.
    /// 3. If the `SyntaxTree` is successfully parsed, it performs semantic analysis using the
    ///    `Analyzer`.
    /// 4. Adds any errors from the syntax's `ErrorManager` to the semantic analysis result's
    ///    `ErrorManager`.
    /// 5. Displays all errors encountered during parsing and analysis.
    ///
    /// If no `SyntaxTree` is produced, it returns an `AnalyzerResult` with no syntax arena and the
    /// errors from the syntax.
    ///
    /// # Arguments
    /// - `source_path`: A string slice representing the path to the source file to be parsed.
    /// - `language`: The `Language` to specify whether to parse as PDDL or HDDL.
    ///
    /// # Returns
    /// - `Ok(AnalyzerResult)`: The result of the semantic analysis, which includes a reference to
    ///   the semantic analysis result.
    /// - `Err(ParserInternalError)`: An error occurred during parsing or analysis.
    ///
    /// # Errors
    /// - If there is an error reading the source file or parsing its content, a
    ///   `ParserInternalError` is returned.
    ///
    /// # Example
    /// ```rust
    /// let result = syntax.parse_file("path/to/source/file.pddl", Language::PDDL);
    /// match result {
    ///     Ok(analysis_result) => { /* Process analysis result */ },
    ///     Err(error) => { /* Handle error */ },
    /// }
    /// ```
    /// Parses, normalizes, and analyzes a source file.
    ///
    /// This function performs the full pipeline:
    /// - Reads the file content,
    /// - Parses the content into an AST,
    /// - Normalizes and checks the AST,
    /// - Performs semantic analysis.
    ///
    /// # Parameters
    /// - `source_path`: The path to the source file.
    ///
    /// # Returns
    /// An `AnalyzerResult` if all steps succeed or partially succeed. Any fatal error
    /// (like file read error or normalization failure) is returned as an `AiplanError`.
    pub fn parse_file(
        &self,
        source_path: &str,
    ) -> Result<AnalyzerResult, AiplanError> {
        // Read the file content
        let content = self.read_file(source_path)?;

        // Parse
        let mut parser = Parser::new();
        let parser_result = parser.parse(source_path, &content)?;

        // Normalize
        let mut normalizer = Normalizer::new();
        let mut normalizer_result = normalizer.normalize(parser_result)?;

        // Analyze
        let mut analyzer = Analyzer::new();
        Ok(analyzer.analyze(&mut normalizer_result)?)
    }

    /// Performs semantic linking between a previously analyzed domain and problem,
    /// both loaded from serialized semantic context files.
    ///
    /// This function:
    /// 1. Deserializes the domain and problem semantic contexts from the given file paths.
    /// 2. Wraps them in `AnalyzerResult` containers with fresh `DiagnosticManager`s.
    /// 3. Performs the semantic linking using the `Linker`.
    ///
    /// # Arguments
    ///
    /// * `lifted_domain_path` - Path to the serialized domain semantic context file (e.g., `.sem` or `.json`).
    /// * `lifted_problem_path` - Path to the serialized problem semantic context file.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkerResult)` if linking was successful or completed with diagnostics.
    /// * `Err(AiplanError)` if deserialization or linking fails catastrophically.
    ///
    /// # Errors
    ///
    /// Returns `Err` if either file fails to deserialize or if the linking logic produces an unrecoverable error.
    pub fn link(
        &self,
        lifted_domain_path: &str,
        lifted_problem_path: &str,
    ) -> Result<LinkerResult, AiplanError> {
        // Deserialize the domain semantic context from file
        let lifted_domain = SemanticContext::deserialize_from_file_with_auto_format(lifted_domain_path)?;
        let domain = AnalyzerResult::success(lifted_domain, DiagnosticManager::new());

        // Deserialize the problem semantic context from file
        let lifted_problem = SemanticContext::deserialize_from_file_with_auto_format(lifted_problem_path)?;
        let problem = AnalyzerResult::success(lifted_problem, DiagnosticManager::new());

        // Perform semantic linking between domain and problem contexts
        let mut linker = Linker::new();
        let linker_result = linker.link(domain, problem)?;

        // Return the linker result (which includes semantic context and diagnostics)
        Ok(linker_result)
    }

    /// Reads the content of a source file into a `String`.
    ///
    /// This function attempts to open the file at the specified path and read its contents into a
    /// `String`. If the file cannot be opened or read, it returns a `ParserInternalError`
    /// describing the problem.
    ///
    /// # Arguments
    /// - `path`: A reference to the `PathBuf` representing the path of the source file to be read.
    ///
    /// # Returns
    /// - `Ok(String)`: The content of the file as a `String` if reading was successful. If the file
    ///   is empty, an empty string will be returned.
    /// - `Err(ParserInternalError)`: An error occurred while opening or reading the file.
    ///
    /// # Errors
    /// - If the file cannot be opened (e.g., it doesn't exist or there are permission issues), a
    ///   `ParserInternalError` with a message indicating the failure to open the file is returned.
    /// - If the file cannot be read (e.g., due to an encoding issue or I/O error), a
    ///   `ParserInternalError` with a message indicating the failure to read the file is returned.
    ///
    /// # Special Cases
    /// - **Empty files**: If the file is empty, this function will return an empty `String`. No
    ///   error is raised in this case.
    /// - **Large files**: This function reads the entire file into memory, which could be
    ///   problematic for very large files. If handling large files, consider reading the file in
    ///   chunks to prevent memory overload.
    /// - **Invalid paths**: If the `path` is invalid (e.g., it points to a non-existent file or a
    ///   directory), an error will be returned indicating that the file could not be opened.
    ///
    /// # Example
    /// ```rust
    /// let path = PathBuf::from("path/to/source/file.pddl");
    /// let result = parser.read_source_file(&path);
    /// match result {
    ///     Ok(content) => { /* Process the content of the file */ },
    ///     Err(error) => { /* Handle error opening or reading the file */ },
    /// }
    /// ```
    /// # Notes
    /// - The file will be read entirely into memory, so this function may not be suitable for very
    ///   large files. For large files, consider using a streaming approach to read the file in
    ///   chunks.
    /// - If the file is empty, an empty `String` will be returned without an error.
    /// - If the file is large, it could put a strain on memory usage. Ensure that the file size is
    ///   manageable for your system.
    fn read_file(&self, path: &str) -> Result<String, AiplanError> {
        // Initialize an empty String to store the file's content.
        let mut source = String::new();

        // Attempt to open the file.
        match File::open(path) {
            // If file is successfully opened, attempt to read its content.
            Ok(mut file) => {
                // If reading the file fails, return a ParserInternalError with the failure message.
                if let Err(e) = file.read_to_string(&mut source) {
                    return Err(AiplanError::InternalError(format!(
                        "Error reading the file: {}",
                        e
                    )));
                }
            }
            // If the file cannot be opened, return a ParserInternalError with the failure message.
            Err(e) => {
                return Err(AiplanError::InternalError(format!(
                    "Error opening the file: {}",
                    e
                )));
            }
        }

        // Return the file's content if reading was successful.
        Ok(source)
    }
}
