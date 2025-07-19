use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::linking::Linker;
use crate::aiplan4rust::linking::LinkerResult;
use crate::aiplan4rust::syntax::{Language, SyntaxDisplay};
use crate::aiplan4rust::syntax::Parser;
use crate::aiplan4rust::semantic::{Analyzer, SemanticContext};
use crate::aiplan4rust::normalization::Normalizer;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeSerializable};

use serde::Deserialize;
use std::backtrace::Backtrace;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::string::String;
use base64::Engine;
use base64::engine::general_purpose;
use crate::aiplan4rust::interner::InternerDisplay;
use crate::aiplan4rust::lir::{LIRBuilder, LIRBuilderResult, LiftedProblem};
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::validation::normalization::check_well_normalized;
use crate::Renderer;

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
    pub fn parse(
        &self,
        domain_path: &str,
        problem_path: &str,
        language: &Language,
    ) -> Result<LIRBuilderResult, ParserInternalError> {
        let mut diagnostic_manager = DiagnosticManager::new();

        // Parse the domain file
        let mut domain = self.parse_file(domain_path, language)?;
        diagnostic_manager.add_diagnostic_from(domain.take_diagnostic_manager());

        // Parse the problem file
        let mut problem = self.parse_file(problem_path, language)?;
        diagnostic_manager.add_diagnostic_from(problem.take_diagnostic_manager());

        match (domain.take_semantic_context(), problem.take_semantic_context()) {
            (Some(domain_tree), Some(problem_tree)) => {

                let mut linker = Linker::new();
                let mut linker_result = linker.link_with_diagnostic_manager(domain_tree, problem_tree, diagnostic_manager)?;

                match linker_result.take_linked_semantic_context() {
                    Some(linked_semantic_context) => {
                        let interner = linked_semantic_context.interner();
                        //println!("**************{}", linked_semantic_context.domain().to_string_with_interner(interner));
                        //println!("Linking successful, building LIR...");
                        //println!("{}", linked_semantic_context.problem().to_planning_string(linked_semantic_context.interner()));
                        let mut ir_builder = LIRBuilder::new();
                        let builder_result = ir_builder.build_with_diagnostic_manager(
                            &linked_semantic_context,
                            linker_result.take_diagnostic_manager()
                        )?;
                        //println!("{}", builder_result.lifted_problem().unwrap().to_string_with_interner(linked_semantic_context.interner()));
 
                        Ok(builder_result)
                    }
                    None => {
                        Ok(LIRBuilderResult::new(None, linker_result.take_diagnostic_manager()))
                    }
                }
            }
            _ => Ok(LIRBuilderResult::new(None, diagnostic_manager)),
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
    pub fn parse_file(
        &self,
        source_path: &str,
        language: &Language,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        // Attempt to read the content of the source file.
        let content = self.read_file(source_path)?;

        // Create a new parser instance.
        let mut parser = Parser::new();

        // Parse the content.
        let mut parser_result = parser.parse(source_path, &content, language)?;

        // Match on the AST extracted from parsing.
        match parser_result.take_ast() {
            Some(raw_ast) => {
                //println!("********************** RAW AST *************************");
                //println!("{}", raw_ast.arena().try_root()?.to_string_with_interner(raw_ast.arena(), raw_ast.interner()));

                //println!("********************** RAW AST *************************");
                //println!("{}", raw_ast.arena().try_root()?.to_syntax_string(raw_ast.arena(), raw_ast.interner()));

                // Take diagnostics from parser result.
                let diagnostic_manager = parser_result.take_diagnostic_manager();

                // Normalize the AST while merging diagnostics.
                let mut normalizer = Normalizer::new();
                let mut normalizer_result =
                    normalizer.normalize_with_diagnostic_manager(raw_ast, diagnostic_manager)?;


                match normalizer_result.take_ast() {
                    Some(mut normalized_ast) => {
                        //println!("********************** NORMALIZED AST *************************");
                        //println!("{}", normalized_ast.arena().try_root()?.to_string_with_interner(normalized_ast.arena(), normalized_ast.interner()));
                        match check_well_normalized(&normalized_ast) {
                            Ok(()) => {
                                println!("AST is well-normalized.");
                            }
                            Err(e) => {
                                println!("{}", normalized_ast.to_string_with_interner());
                                panic!("AST normalization check failed: {}", e);
                            }
                        }

                        let interner = normalized_ast.interner();
                        // Retrieve diagnostics accumulated during normalization.
                        let diagnostic_manager = normalizer_result.take_diagnostic_manager();
                        // Analyze the normalized AST with the diagnostics.
                        let mut analyzer = Analyzer::new();
                        let analysis_result =
                            analyzer.analyze_with_diagnostic_manager(&mut normalized_ast, diagnostic_manager)?;

                        // Return the analysis result.
                        Ok(analysis_result)
                    }
                    None => Self::create_error_result(normalizer_result.diagnostic_manager_mut()),
                }
            }
            //None => Self::create_error_result(parser_result.diagnostic_manager_mut()),
            None => Self::create_error_result(parser_result.diagnostic_manager_mut())

        }


    }

    fn create_error_result(
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        Ok(AnalyzerResult::new(None, std::mem::take(diagnostic_manager)))
    }

    pub fn link(
        &self,
        lifted_domain_path: &str,
        lifted_problem_path: &str,
    ) -> Result<LinkerResult, ParserInternalError> {
        let diagnostic_manager = DiagnosticManager::new();

        // Désérialiser le fichier de domaine
        let lifted_domain = SemanticContext::deserialize_from_file_auto_format(lifted_domain_path)?;


        // Désérialiser le fichier de problème
        let lifted_problem = SemanticContext::deserialize_from_file_auto_format(lifted_problem_path)?;

        // Tentative de linking entre le domain et le problem
        let mut linker = Linker::new();
        let mut linker_result = linker.link_with_diagnostic_manager(
            lifted_domain,
            lifted_problem,
            diagnostic_manager
        )?;

        // Retourner le résultat avec l'état de l'erreur accumulée
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
    fn read_file(&self, path: &str) -> Result<String, ParserInternalError> {
        // Initialize an empty String to store the file's content.
        let mut source = String::new();

        // Attempt to open the file.
        match File::open(path) {
            // If file is successfully opened, attempt to read its content.
            Ok(mut file) => {
                // If reading the file fails, return a ParserInternalError with the failure message.
                if let Err(e) = file.read_to_string(&mut source) {
                    return Err(ParserInternalError::new(format!(
                        "Error reading the file: {}",
                        e
                    )));
                }
            }
            // If the file cannot be opened, return a ParserInternalError with the failure message.
            Err(e) => {
                return Err(ParserInternalError::new(format!(
                    "Error opening the file: {}",
                    e
                )));
            }
        }

        // Return the file's content if reading was successful.
        Ok(source)
    }
}

#[derive(Debug)]
pub struct ParserInternalError {
    pub message: String,
    pub backtrace: Backtrace,
}

impl ParserInternalError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            backtrace: Backtrace::capture(),
        }
    }
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for ParserInternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ParserInternalError : {}", self.message)?;

        // Affiche la backtrace seulement en mode debug
        if cfg!(debug_assertions) {
            writeln!(f, "Backtrace :\n{}", self.backtrace)?;
        }

        Ok(())
    }
}
impl std::error::Error for ParserInternalError {}
