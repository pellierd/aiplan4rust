#![allow(dead_code)]

use crate::common::io::{
    read_file, write_ast_to_file, write_diagnostics_to_file, write_error_diagnostic_file,
    write_error_diagnostic_file_for_domain_and_problem, write_linking_diag_to_file,
    write_symbol_table_to_file,
};
use aiplan4rust::aiplan4rust::linking::LinkerResult;
use aiplan4rust::aiplan4rust::normalization::NormalizerResult;
use aiplan4rust::aiplan4rust::syntax::ParserResult;
use aiplan4rust::aiplan4rust::validation::normalization::check_well_normalized;
use aiplan4rust::aiplan4rust::{Analyzer, Linker};
use aiplan4rust::{check_well_formed, AnalyzerResult, Language, Normalizer, Parser, Severity};
use std::path::Path;

/// Parses the source file to produce a ParserResult with a raw AST and checks its well-formedness.
///
/// This function reads the source file, parses its content into a `ParserResult` using the specified language,
/// and then validates that the resulting AST, if present, is well-formed. It manages diagnostic messages throughout
/// the process and writes diagnostic and AST files on errors to assist debugging.
///
/// # Arguments
///
/// * `file_path` - The path to the source file to be parsed.
/// * `language` - The language specification used by the parser.
///
/// # Returns
///
/// Returns `Some(ParserResult)` if parsing succeeded and the AST (if present) is well-formed.
/// Returns `None` if parsing failed, no AST was produced, or if the AST fails the well-formedness check.
///
/// # Side Effects
///
/// - Writes diagnostic files for both parse errors and AST validation errors.
/// - Writes the raw AST to a file if validation fails.
///
/// # Panics
///
/// This function will panic if the file path is not valid UTF-8.
///
/// # Example
///
/// ```rust
/// if let Some(parser_result) = parse_and_check_ast(&file_path, &language) {
///     // proceed with parser_result
/// } else {
///     // handle parse or validation failure
/// }
/// ```
pub fn parse_and_check_ast(file_path: &Path, language: &Language) -> Option<ParserResult> {
    let content = read_file(file_path);
    let path_str = file_path.to_str().expect("File path is not valid UTF-8");
    let mut parser = Parser::new();

    match parser.parse(path_str, &content, language) {
        Ok(parser_result) => {
            if let Some(raw_ast) = parser_result.ast() {
                if let Err(e) = check_well_formed(raw_ast) {
                    eprintln!(
                        "Raw AST validation failed for {}:\n{}",
                        file_path.display(),
                        e
                    );
                    write_diagnostics_to_file(
                        parser_result.diagnostic_manager(),
                        parser_result.interner(),
                        file_path,
                        "Raw AST validation error",
                    );
                    write_ast_to_file(raw_ast, file_path, "Raw AST validation error");
                    return None;
                }
                Some(parser_result)
            } else {
                eprintln!("Parsing failed (no AST) for file {}", file_path.display());
                write_diagnostics_to_file(
                    parser_result.diagnostic_manager(),
                    parser_result.interner(),
                    file_path,
                    "Parsing failed (no AST)",
                );
                None
            }
        }
        Err(e) => {
            eprintln!("Parsing error for file {}: {}", file_path.display(), e);
            write_error_diagnostic_file(file_path, "Parsing error", &e.to_string());
            None
        }
    }
}

/// Normalizes the AST contained in a `ParserResult` and checks its correctness,
/// returning the normalization result.
///
/// This function runs the normalization process on the AST extracted from the given
/// `ParserResult`, validating the resulting normalized AST.
/// It writes diagnostic information and, in case of errors, writes the normalized AST
/// and diagnostic details to files to facilitate debugging.
///
/// # Arguments
///
/// * `parser_result` - The result of parsing, containing the raw AST, diagnostics, and interner.
/// * `file_path` - The path to the source file associated with the AST, used for naming output files.
///
/// # Returns
///
/// Returns `Some(NormalizerResult)` if normalization succeeds without blocking errors.
/// Returns `None` if normalization fails or errors are found.
///
/// # Side Effects
///
/// - Writes diagnostic files for both success and error cases.
/// - Writes the normalized AST to a file if validation fails or no AST is produced.
/// - Writes an error diagnostic file on normalization failure.
///
/// # Panics
///
/// This function will panic if writing to any output files fails.
///
/// # Example
///
/// ```rust
/// if let Some(normalizer_result) = normalize_and_check_ast(parser_result, &file_path) {
///     // proceed with normalizer_result
/// } else {
///     // handle normalization failure
/// }
/// ```
pub fn normalize_and_check_ast(
    parser_result: ParserResult,
    file_path: &Path,
) -> Option<NormalizerResult> {
    let mut normalizer = Normalizer::new();

    match normalizer.normalize(parser_result) {
        Ok(normalizer_result) => {
            if let Some(normalized_ast) = normalizer_result.ast() {
                if let Err(e) = check_well_normalized(&normalized_ast) {
                    eprintln!(
                        "Normalized AST validation failed for {}:\n{}",
                        file_path.display(),
                        e
                    );
                    eprintln!("{}", normalized_ast.to_string_with_interner());
                    write_diagnostics_to_file(
                        normalizer_result.diagnostic_manager(),
                        normalizer_result.interner(),
                        file_path,
                        "Normalized AST validation error",
                    );
                    write_ast_to_file(
                        &normalized_ast,
                        file_path,
                        "Normalized AST validation error",
                    );
                    return None;
                } else {
                    write_diagnostics_to_file(
                        normalizer_result.diagnostic_manager(),
                        normalizer_result.interner(),
                        file_path,
                        "Normalization success",
                    );
                }

                let diag_mgr = normalizer_result.diagnostic_manager();
                if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                    eprintln!("Diagnostics errors found for file {}", file_path.display());
                    return None;
                }

                Some(normalizer_result)
            } else {
                eprintln!(
                    "No normalized AST returned for file {}",
                    file_path.display()
                );
                write_diagnostics_to_file(
                    normalizer_result.diagnostic_manager(),
                    normalizer_result.interner(),
                    file_path,
                    "Normalized AST validation failed (no AST)",
                );
                None
            }
        }
        Err(e) => {
            eprintln!(
                "Normalization error for file {}: {}",
                file_path.display(),
                e
            );
            write_error_diagnostic_file(
                file_path,
                "Normalizer tests: Normalization error",
                &e.to_string(),
            );
            None
        }
    }
}

/// Performs semantic analysis using the provided NormalizerResult.
///
/// # Parameters
/// - `normalizer_result`: Contains the normalized AST, diagnostics, and interner.
/// - `file_path`: Used for logging and writing output files.
///
/// # Returns
/// - `Some(AnalyzerResult)` if semantic analysis succeeded without blocking errors.
/// - `None` if semantic analysis failed or reported blocking diagnostics.
pub fn analyze(
    mut normalizer_result: NormalizerResult,
    file_path: &Path,
) -> Option<AnalyzerResult> {
    let mut analyzer = Analyzer::new();

    match analyzer.analyze(&mut normalizer_result) {
        Ok(analyzer_result) => {
            let diag_mgr = analyzer_result.diagnostic_manager();

            write_diagnostics_to_file(
                diag_mgr,
                analyzer_result.interner(),
                file_path,
                "Analyzer tests: Semantic analysis result",
            );

            if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                eprintln!(
                    "Semantic analysis reported errors for file {}",
                    file_path.display()
                );

                // Write AST if available
                if let Some(ast) = normalizer_result.ast() {
                    write_ast_to_file(ast, file_path, "Analyzer error");

                    // Write symbol table if context exists
                    if let Some(sem_ctx) = analyzer_result.semantic_context() {
                        let symtab = sem_ctx.symbol_table();
                        write_symbol_table_to_file(
                            symtab,
                            file_path,
                            "Analyzer error",
                            ast.interner(),
                        );
                    }
                }

                None
            } else {
                Some(analyzer_result)
            }
        }
        Err(e) => {
            eprintln!(
                "Semantic analysis failed for {}: {}",
                file_path.display(),
                e
            );

            write_error_diagnostic_file(
                file_path,
                "Analyzer tests: Semantic analysis error",
                &e.to_string(),
            );

            // Write AST even on panic
            if let Some(ast) = normalizer_result.ast() {
                write_ast_to_file(ast, file_path, "Analyzer panic");
            }

            None
        }
    }
}

/// Parses, normalizes, and performs semantic analysis on a file.
///
/// This function processes the file at `file_path` using the specified `language`,
/// and goes through the following three stages:
///
/// 1. **Parsing**: Converts the file contents into a raw AST with initial diagnostics.
/// 2. **Normalization**: Transforms and validates the AST.
/// 3. **Semantic analysis**: Builds the semantic context and final diagnostics.
///
/// The `context` parameter is used in error messages and diagnostics (e.g., "domain" or "problem").
///
/// The `success` flag will be set to `false` if any stage fails (parse, normalize, or analyze).
///
/// # Arguments
///
/// * `file_path` - Path to the file being processed.
/// * `language` - Language definition used for parsing and validation.
/// * `context` - Human-readable context used for error logging.
/// * `success` - Mutable flag to indicate whether the analysis was fully successful.
///
/// # Returns
///
/// Returns `Some(AnalyzerResult)` on full success, or `None` if any stage fails.
/// On failure, `success` is set to `false` and errors are printed to stderr.
pub fn analyze_file(
    file_path: &Path,
    language: &Language,
    context: &str,      // e.g., "domain" or "problem"
    success: &mut bool, // mutable reference to update success flag
) -> Option<AnalyzerResult> {
    // Step 1: Parse the source file into an AST + diagnostics
    let parser_result = match parse_and_check_ast(file_path, language) {
        Some(result) => result,
        None => {
            eprintln!("Parsing failed for {}: {}", context, file_path.display());
            *success = false;
            return None;
        }
    };

    // Step 2: Normalize the parsed AST
    let normalizer_result = match normalize_and_check_ast(parser_result, file_path) {
        Some(result) => result,
        None => {
            eprintln!(
                "Normalization failed for {}: {}",
                context,
                file_path.display()
            );
            *success = false;
            return None;
        }
    };

    // Step 3: Perform semantic analysis
    let analyzer_result = match analyze(normalizer_result, file_path) {
        Some(result) => result,
        None => {
            eprintln!(
                "Semantic analysis failed for {}: {}",
                context,
                file_path.display()
            );
            *success = false;
            return None;
        }
    };

    Some(analyzer_result)
}

/// Performs semantic linking between a domain and a problem.
///
/// This function takes the results of semantic analysis (`AnalyzerResult`) for both
/// the domain and the problem, performs semantic linking using the [`Linker`], and
/// returns a [`LinkerResult`] if the linking process completes successfully without
/// structural errors.
///
/// # Parameters
///
/// - `domain`: The `AnalyzerResult` of the domain file (already parsed, normalized, and analyzed).
/// - `problem`: The `AnalyzerResult` of the problem file (already parsed, normalized, and analyzed).
/// - `domain_path`: Path to the domain source file. Used to name the diagnostic output files.
/// - `problem_path`: Path to the problem source file. Used to name the diagnostic output files.
///
/// # Returns
///
/// - `Some(LinkerResult)`: If linking completes successfully and produces a result,
///   even if that result contains warnings or errors.
/// - `None`: If a structural error occurs during linking, such as:
///   - internal linking failure,
///   - invalid or missing semantic context,
///   - failure to merge identifier spaces.
///
/// # Behavior
///
/// - Linking is performed via [`Linker::link`], which consumes both `AnalyzerResult`s.
/// - All diagnostics generated during linking are written to disk as a `.linking.diag` file.
/// - If semantic context is missing, a dedicated `.error.diag` file is also written.
/// - If error-level diagnostics are present, the result is still written, but `None` is returned
///   to indicate linking failure in the calling context.
///
/// # Side Effects
///
/// Writes one or more diagnostic files to disk in the same directory as `domain_path`:
/// - `<problem>-<domain>.linking.diag`: Contains all diagnostics from the linking process.
/// - `<problem>-<domain>.error.diag`: If linking failed due to missing semantic context.
///
/// # Example
///
/// ```ignore
/// let domain_result = analyzer.analyze(...)?;
/// let problem_result = analyzer.analyze(...)?;
///
/// let result = link(domain_result, problem_result, Path::new("domain.hddl"), Path::new("problem.hddl"));
///
/// match result {
///     Some(linker_result) => {
///         if linker_result.diagnostic_manager().has_diagnostics_of_severity(Severity::Error) {
///             eprintln!("Linking completed with errors.");
///         } else {
///             println!("Linking completed successfully.");
///         }
///     }
///     None => {
///         eprintln!("Linking failed due to structural errors or missing semantic context.");
///     }
/// }
/// ```
///
/// # See Also
/// - [`Linker`]
/// - [`AnalyzerResult`]
/// - [`LinkerResult`]
pub fn link(
    domain: AnalyzerResult,
    problem: AnalyzerResult,
    domain_path: &Path,
    problem_path: &Path,
) -> Option<LinkerResult> {
    let mut linker = Linker::new();

    // Attempt linking; convert structural errors to None
    let linker_result = linker.link(domain, problem).ok()?;

    // Always write diagnostics
    write_linking_diag_to_file(
        &linker_result.diagnostic_manager(),
        linker_result.interner(),
        domain_path,
        problem_path,
        "Linking tests",
    );

    // Handle missing linked context
    if linker_result.linked_semantic_context().is_none() {
        eprintln!(
            "Linking failed for domain {} and problem {}: missing semantic context",
            domain_path.display(),
            problem_path.display()
        );

        write_error_diagnostic_file_for_domain_and_problem(
            domain_path,
            problem_path,
            "Linking error",
            "Missing semantic context",
        );

        return None;
    }

    // Log and fail if hard errors are present
    if linker_result
        .diagnostic_manager()
        .has_diagnostics_of_severity(Severity::Error)
    {
        eprintln!(
            "Linking reported errors for file {}",
            problem_path.display()
        );
        return None;
    }

    Some(linker_result)
}
