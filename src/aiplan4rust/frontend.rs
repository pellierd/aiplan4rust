//! Frontend module for AI planning pipeline.
//!
//! This module provides the `Frontend` struct, which serves as the main entry point
//! for parsing, linking, and building intermediate representations (LIR) from domain
//! and problem files. It orchestrates the full AI planning compilation pipeline:
//! reading input files, parsing them, performing semantic analysis, linking domain
//! and problem contexts, and optionally generating serialized outputs.
//!
//! # Example
//! ```rust
//! use aiplan4rust::frontend::Frontend;
//!
//! let frontend = Frontend::new();
//! // Use frontend to parse and link planning files
//! ```

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::io::input::Input;
use crate::aiplan4rust::linking::Linker;
use crate::aiplan4rust::lir::{LirBuilder, LirBuilderResult};
use crate::aiplan4rust::normalization::Normalizer;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::semantic::{Analyzer, SemanticContext};
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::Parser;
use crate::aiplan4rust::AiplanError;

/// Frontend struct for the AI planning pipeline.
///
/// `Frontend` provides high-level methods to:
/// - Parse raw domain and problem inputs,
/// - Perform semantic analysis and normalization,
/// - Link domain and problem contexts,
/// - Build intermediate representations (LIR),
/// - Serialize or return diagnostics if errors occur.
///
/// This struct acts as the main interface for CLI commands like `parse` and `link`.
///
/// # Example
/// ```rust
/// use aiplan4rust::frontend::Frontend;
///
/// let frontend = Frontend::new();
/// ```
#[derive(Debug)]
pub struct Frontend {}

impl Frontend {
    /// Creates a new instance of the `Frontend`.
    ///
    /// This function initializes the `Frontend` struct, which provides methods for parsing,
    /// linking, and building intermediate representations (LIR) from domain and problem inputs.
    ///
    /// # Returns
    /// - A new `Frontend` instance ready for use.
    ///
    /// # Example
    /// ```rust
    /// use aiplan4rust::frontend::Frontend;
    ///
    /// let frontend = Frontend::new();
    /// ```
    pub fn new() -> Self {
        Self {}
    }

    /// Parses, normalizes, and semantically analyzes a raw input source file.
    ///
    /// This function performs the full pipeline on an input source that represents **raw PDDL or HDDL content**:
    /// 1. Reads and parses the content into an abstract syntax tree (AST) using the `Parser`.
    /// 2. Normalizes and validates the AST using the `Normalizer`.
    /// 3. Performs semantic analysis using the `Analyzer`, producing an `AnalyzerResult`.
    ///
    /// # Input
    /// - `input`: A reference to an `Input` instance, **expected to be of kind `Input::Raw`**.
    ///   This means the content has not yet been parsed or lifted into an intermediate representation (IR).
    ///   Using other `Input` kinds may produce errors or unintended behavior.
    ///
    /// # Returns
    /// - `Ok(AnalyzerResult)`: Contains the results of the semantic analysis, including a semantic context,
    ///   diagnostics, and symbol interning.
    /// - `Err(AiplanError)`: Indicates an error occurred during parsing, normalization, or analysis.
    ///
    /// # Errors
    /// Returns an `AiplanError` if:
    /// - Reading the source file fails.
    /// - Parsing or normalization fails.
    /// - Semantic analysis encounters a fatal error.
    ///
    /// # Example
    /// ```rust
    /// use aiplan4rust::io::input::Input;
    /// use aiplan4rust::frontend::Frontend;
    ///
    /// let frontend = Frontend::new();
    /// let input = Input::read_from_file("domain.pddl")?; // must be raw
    /// let analysis_result = frontend.parse_from_raw_input(&input)?;
    /// ```
    pub fn parse_from_raw_input(&self, input: &Input) -> Result<AnalyzerResult, AiplanError> {
        // Step 1: Parse
        let mut parser = Parser::new();
        let parser_result = parser.parse(input)?;

        // Step 2: Normalize
        let mut normalizer = Normalizer::new();
        let mut normalizer_result = normalizer.normalize(parser_result)?;

        // Step 3: Semantic analysis
        let mut analyzer = Analyzer::new();
        Ok(analyzer.analyze(&mut normalizer_result)?)
    }

    /// Links and builds a LIR from raw domain and problem inputs.
    ///
    /// This function performs the full pipeline starting from **raw input sources**:
    /// 1. Parses, normalizes, and semantically analyzes the domain and problem using `parse_from_raw_input`.
    /// 2. Performs semantic linking between the analyzed domain and problem using `Linker`.
    /// 3. If linking succeeds, builds a **Lifted Intermediate Representation (LIR)** using `LirBuilder`.
    ///
    /// # Input
    /// - `domain_input`: A reference to an `Input` representing the **raw domain file**.
    /// - `problem_input`: A reference to an `Input` representing the **raw problem file**.
    /// Both inputs are expected to be of kind `Input::Raw`.
    ///
    /// # Returns
    /// - `Ok(LirBuilderResult)`: On success, contains the built LIR or diagnostics if linking/analysis partially failed.
    /// - `Err(AiplanError)`: If parsing, normalization, analysis, or linking fails catastrophically.
    ///
    /// # Errors
    /// Returns an `AiplanError` if:
    /// - Parsing or semantic analysis fails for either the domain or problem.
    /// - Semantic linking fails due to inconsistencies or fatal errors.
    ///
    /// # Example
    /// ```rust
    /// use aiplan4rust::io::input::Input;
    /// use aiplan4rust::frontend::Frontend;
    ///
    /// let frontend = Frontend::new();
    /// let domain_input = Input::read_from_file("domain.pddl")?; // must be raw
    /// let problem_input = Input::read_from_file("problem.pddl")?; // must be raw
    ///
    /// let lir_result = frontend.link_from_raw_input(&domain_input, &problem_input)?;
    /// ```
    pub fn link_from_raw_input(
        &self,
        domain_input: &Input,
        problem_input: &Input,
    ) -> Result<LirBuilderResult, AiplanError> {
        // Step 1: Parse, normalize, and analyze both domain and problem files
        let domain = self.parse_from_raw_input(&domain_input)?;
        let problem = self.parse_from_raw_input(&problem_input)?;

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
            // Linking failed: return diagnostics and interner only
            let diagnostic_manager = linker_result.take_diagnostic_manager();
            let interner = linker_result.take_interner();
            Ok(LirBuilderResult::failure(diagnostic_manager, interner))
        }
    }

    /// Links and builds a LIR from previously parsed or serialized domain and problem inputs.
    ///
    /// This function performs the full pipeline starting from **parsed or serialized semantic contexts**:
    /// 1. Deserializes the domain and problem semantic contexts from the given `Input` files.
    /// 2. Wraps them in `AnalyzerResult` containers with fresh `DiagnosticManager`s.
    /// 3. Performs semantic linking between the domain and problem using `Linker`.
    /// 4. If linking succeeds, builds a **Lifted Intermediate Representation (LIR)** using `LirBuilder`.
    ///
    /// # Input
    /// - `domain`: A reference to an `Input` containing a **parsed/serialized domain** (e.g., `.sem`, `.json`).
    /// - `problem`: A reference to an `Input` containing a **parsed/serialized problem** (e.g., `.sem`, `.json`).
    /// Both inputs are expected to be of kind `Input::IR` or other serialized formats supported by `SemanticContext::deserialize_from_file_with_auto_format`.
    ///
    /// # Returns
    /// - `Ok(LirBuilderResult)`: On success, contains the built LIR or diagnostics if linking partially failed.
    /// - `Err(AiplanError)`: If deserialization or linking fails catastrophically.
    ///
    /// # Errors
    /// Returns an `AiplanError` if:
    /// - Deserialization of either the domain or problem fails.
    /// - Semantic linking fails due to inconsistencies or fatal errors.
    ///
    /// # Example
    /// ```rust
    /// use aiplan4rust::io::input::Input;
    /// use aiplan4rust::frontend::Frontend;
    ///
    /// let frontend = Frontend::new();
    /// let domain_input = Input::read_from_file("domain.sem")?;
    /// let problem_input = Input::read_from_file("problem.sem")?;
    ///
    /// let lir_result = frontend.link_from_parsed_input(&domain_input, &problem_input)?;
    /// ```
    pub fn link_from_parsed_input(
        &self,
        domain: &Input,
        problem: &Input,
    ) -> Result<LirBuilderResult, AiplanError> {
        // 1. Deserialize semantic contexts
        let lifted_domain = SemanticContext::deserialize_from_file_with_auto_format(domain.path())?;
        let lifted_problem =
            SemanticContext::deserialize_from_file_with_auto_format(problem.path())?;

        let domain = AnalyzerResult::success(lifted_domain, DiagnosticManager::new());
        let problem = AnalyzerResult::success(lifted_problem, DiagnosticManager::new());

        // 2. Link
        let mut linker = Linker::new();
        let mut linker_result = linker.link(domain, problem)?;

        // 3. Build LIR if possible
        if let Some(mut linked_context) = linker_result.take_linked_semantic_context() {
            let mut lir_builder = LirBuilder::new();
            let lir_result = lir_builder.build_with_diagnostic_manager(
                &mut linked_context,
                linker_result.take_diagnostic_manager(),
            )?;
            Ok(lir_result)
        } else {
            // Linking failed → return diagnostics
            Ok(LirBuilderResult::failure(
                linker_result.take_diagnostic_manager(),
                linker_result.take_interner(),
            ))
        }
    }
}
