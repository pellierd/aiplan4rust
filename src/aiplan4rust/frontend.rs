use crate::aiplan4rust::error::ErrorManager;
use crate::aiplan4rust::linker::LiftedPlanningTask;
use crate::aiplan4rust::linker::Linker;
use crate::aiplan4rust::linker::LinkerResult;
use crate::aiplan4rust::parser::Language;
use crate::aiplan4rust::parser::Parser;
use crate::aiplan4rust::semantic_analyser::semantic_analyzer::SemanticAnalyzer;
use crate::aiplan4rust::semantic_analyser::AnalyzerResult;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::LiftedDomain;
use crate::aiplan4rust::semantic_analyser::LiftedProblem;
use crate::aiplan4rust::FileFormat;

use serde::Deserialize;
use std::backtrace::Backtrace;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::mem;
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
    /// - If the syntax tree of the domain or problem is `None`, an error is returned.
    /// - If the linking process fails, the error is returned.
    pub fn parse(
        &self,
        domain_path: &str,
        problem_path: &str,
        language: &Language,
    ) -> Result<LinkerResult, ParserInternalError> {
        let error_manager = ErrorManager::new();

        // Parse the domain file
        let domain = self.parse_file(domain_path, language)?;

        // Parse the problem file
        let problem = self.parse_file(problem_path, language)?;

        // Check if the annotated syntax tree exists for the domain
        let domain_tree = match domain.annotated_syntax_tree() {
            Some(tree) => tree,
            None => {
                return Ok(LinkerResult::new(None, error_manager));
            }
        };

        // Check if the annotated syntax tree exists for the problem
        let problem_tree = match problem.annotated_syntax_tree() {
            Some(tree) => tree,
            None => {
                return Ok(LinkerResult::new(None, error_manager));
            }
        };

        // Attempt to link the domain and problem
        let mut linker = Linker::new();
        let mut linker_result = linker.link(domain_tree, problem_tree)?;

        // Add all errors from the error manager into the linker result
        linker_result
            .error_manager_mut()
            .add_errors_from(&error_manager);

        // Return the result with the accumulated error state
        Ok(linker_result)
    }

    /// Parses the source file at the given path and performs semantic analysis on the parsed syntax
    /// tree.
    ///
    /// This function performs the following steps:
    /// 1. Reads the content of the source file specified by `source_path`.
    /// 2. Uses the `Parser` to parse the content into a `SyntaxTree`.
    /// 3. If the `SyntaxTree` is successfully parsed, it performs semantic analysis using the
    ///    `Analyzer`.
    /// 4. Adds any errors from the parser's `ErrorManager` to the semantic analysis result's
    ///    `ErrorManager`.
    /// 5. Displays all errors encountered during parsing and analysis.
    ///
    /// If no `SyntaxTree` is produced, it returns an `AnalyzerResult` with no syntax tree and the
    /// errors from the parser.
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
    /// let result = parser.parse_file("path/to/source/file.pddl", Language::PDDL);
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

        // Attempt to parse the content, returning the result in parser_result.
        let mut parser_result = parser.parse(source_path, &content, language)?;

        // Match on the syntax tree from the parser result.
        match parser_result.syntax_tree() {
            // If the syntax tree is present, perform semantic analysis.
            Some(syntax_tree) => {
                // Create a new analyzer and perform semantic analysis on the syntax tree.
                let mut analyzer = SemanticAnalyzer::new();
                let mut analysis_result = analyzer.analyze(syntax_tree)?;

                // Add errors from the parser's error manager to the analysis result.
                analysis_result
                    .error_manager_mut()
                    .add_errors_from(&parser_result.error_manager());

                // Return the semantic analysis result.
                Ok(analysis_result)
            }
            // If no syntax tree is available, return an analysis result with errors.
            None => Ok(AnalyzerResult::new(
                None,
                mem::take(&mut parser_result.error_manager_mut()),
            )),
        }
    }

    pub fn link(
        &self,
        lifted_domain_path: &str,
        lifted_problem_path: &str,
    ) -> Result<LinkerResult, ParserInternalError> {
        let error_manager = ErrorManager::new();

        // Désérialiser le fichier de domaine
        let lifted_domain = self.deserialize_domain_from_file(lifted_domain_path)?;

        // Désérialiser le fichier de problème
        let lifted_problem = self.deserialize_problem_from_file(lifted_problem_path)?;

        // Tentative de linking entre le domain et le problem
        let mut linker = Linker::new();
        let mut linker_result = linker.link(&lifted_domain, &lifted_problem)?;

        // Ajouter toutes les erreurs du gestionnaire d'erreurs dans le résultat du linker
        linker_result
            .error_manager_mut()
            .add_errors_from(&error_manager);

        // Retourner le résultat avec l'état de l'erreur accumulée
        Ok(linker_result)
    }

    // Fonction générique pour essayer de parser en JSON ou YAML
    fn try_parse_pddl_file<T: for<'de> Deserialize<'de>>(
        content: &str,
        format: FileFormat,
    ) -> Result<T, Box<dyn std::error::Error>> {
        match format {
            FileFormat::Json => serde_json::from_str(content).map_err(Into::into),
            FileFormat::Yaml => serde_yaml::from_str(content).map_err(Into::into),
        }
    }

    // Désérialise un LiftedPlanningTask depuis une chaîne JSON ou YAML
    fn try_parse_lifted_task<T: for<'de> Deserialize<'de>>(
        content: &str,
    ) -> Result<T, ParserInternalError> {
        serde_json::from_str(content)
            .or_else(|_| serde_yaml::from_str(content))
            .map_err(|_| ParserInternalError::new(
                "Failed to parse the planning task file! The content is neither valid JSON nor YAML."
                    .to_string(),
            ))
    }

    // Désérialise un LiftedPlanningTask depuis un fichier en détectant le format
    pub fn deserialize_planning_task_from_file(
        &self,
        file: &str,
    ) -> Result<LiftedPlanningTask, ParserInternalError> {
        let content = self.read_file(file)?;
        Self::try_parse_lifted_task(&content)
    }

    // Deserialize a domain from a file, detecting format automatically
    pub fn deserialize_domain_from_file(
        &self,
        file: &str,
    ) -> Result<LiftedDomain, ParserInternalError> {
        let content = self.read_file(file)?;

        // Try parsing as JSON first
        match Self::try_parse_pddl_file::<LiftedDomain>(&content, FileFormat::Json) {
            Ok(domain) => Ok(domain),
            Err(_) => match Self::try_parse_pddl_file::<LiftedDomain>(&content, FileFormat::Yaml) {
                Ok(domain) => Ok(domain),
                Err(_) => Err(ParserInternalError::new(
                    "Failed to parse the domain file! The content is neither valid JSON nor YAML."
                        .to_string(),
                )),
            },
        }
    }

    // Sérialise un LiftedPlanningTask en chaîne JSON ou YAML
    pub fn serialize_planning_task_to_string(
        &self,
        task: &LiftedPlanningTask,
        format: &FileFormat,
    ) -> Result<String, ParserInternalError> {
        match format {
            FileFormat::Json => serde_json::to_string_pretty(task)
                .map_err(|e| ParserInternalError::new(format!("Error serializing to JSON: {}", e))),
            FileFormat::Yaml => serde_yaml::to_string(task)
                .map_err(|e| ParserInternalError::new(format!("Error serializing to YAML: {}", e))),
        }
    }

    // Sérialise un LiftedPlanningTask dans un fichier JSON ou YAML
    pub fn serialize_planning_task_to_file(
        &self,
        task: &LiftedPlanningTask,
        format: &FileFormat,
        output_file: &str,
    ) -> Result<(), ParserInternalError> {
        let serialized_data = self.serialize_planning_task_to_string(task, format)?;
        std::fs::write(output_file, serialized_data)
            .map_err(|e| ParserInternalError::new(format!("Unable to write file: {}", e)))?;
        Ok(())
    }

    // Deserialize a problem from a file, detecting format automatically
    pub fn deserialize_problem_from_file(
        &self,
        file: &str,
    ) -> Result<LiftedProblem, ParserInternalError> {
        let content = self.read_file(file)?;

        // Try parsing as JSON first
        match Self::try_parse_pddl_file::<LiftedProblem>(&content, FileFormat::Json) {
            Ok(problem) => Ok(problem),
            Err(_) => match Self::try_parse_pddl_file::<LiftedProblem>(&content, FileFormat::Yaml) {
                Ok(problem) => Ok(problem),
                Err(_) => Err(ParserInternalError::new(
                    "Failed to parse the problem file! The content is neither valid JSON nor YAML."
                        .to_string(),
                )),
            },
        }
    }

    /// Serialize the AnnotatedSyntaxTree to a string in the specified format
    pub fn serialize_to_string(
        &self,
        data: &AnnotatedSyntaxTree,
        format: &FileFormat,
    ) -> Result<String, ParserInternalError> {
        match format {
            FileFormat::Json => serde_json::to_string_pretty(data)
                .map_err(|e| ParserInternalError::new(format!("Error serializing to JSON: {}", e))),
            FileFormat::Yaml => serde_yaml::to_string(data)
                .map_err(|e| ParserInternalError::new(format!("Error serializing to YAML: {}", e))),
        }
    }

    // Serialize the AnnotatedSyntaxTree to a file (JSON or YAML)
    pub fn serialize_to_file(
        &self,
        data: &AnnotatedSyntaxTree,
        format: &FileFormat,
        output_file: &str,
    ) -> Result<(), ParserInternalError> {
        let serialized_data = self.serialize_to_string(data, format)?;
        std::fs::write(output_file, serialized_data)
            .map_err(|e| ParserInternalError::new(format!("Unable to write file: {}", e)))?;
        Ok(())
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ParserInternalError: {} \n{}",
            self.message(),
            self.backtrace()
        )
    }
}

impl std::error::Error for ParserInternalError {}
