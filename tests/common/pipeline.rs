use crate::common::io::{read_file, write_ast_to_file, write_diagnostics_to_file, write_error_diagnostic_file, write_error_diagnostic_file_for_domain_and_problem, write_linking_diag_to_file, write_symbol_table_to_file};
use aiplan4rust::aiplan4rust::diagnostic::DiagnosticManager;
use aiplan4rust::aiplan4rust::semantic::SemanticContext;
use aiplan4rust::aiplan4rust::syntax::ast::Ast;
use aiplan4rust::aiplan4rust::validation::normalization::check_well_normalized;
use aiplan4rust::aiplan4rust::{Analyzer, Linker};
use aiplan4rust::{check_well_formed, Language, Normalizer, Parser, Severity};
use std::path::Path;
use aiplan4rust::aiplan4rust::linking::LinkedSemanticContext;

/// Parses the source file to produce a raw AST and checks its well-formedness.
///
/// This function reads the source file, parses its content into an AST using the specified language,
/// and then validates that the resulting AST is well-formed. It also manages diagnostic messages throughout
/// the process and writes diagnostic and AST files on errors to assist debugging.
///
/// # Arguments
///
/// * `file_path` - The path to the source file to be parsed.
/// * `language` - The language specification used by the parser.
///
/// # Returns
///
/// Returns `Some((Ast, DiagnosticManager))` with the raw AST and the diagnostic manager if parsing and validation succeed.
/// Returns `None` if parsing fails, no AST is produced, or if the AST fails the well-formedness check.l11
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
/// if let Some((raw_ast, diag_manager)) = parse_and_check_ast(&file_path, &language) {
///     // proceed with raw AST
/// } else {
///     // handle parse or validation failure
/// }
/// ```
pub fn parse_and_check_ast(
    file_path: &Path,
    language: &Language,
) -> Option<(Ast, DiagnosticManager)> {
    let content = read_file(file_path);
    let path_str = file_path.to_str().expect("File path is not valid UTF-8");
    let mut parser = Parser::new();

    match parser.parse(path_str, &content, language) {
        Ok(mut parser_result) => {
            if let Some(raw_ast) = parser_result.take_ast() {
                if let Err(e) = check_well_formed(&raw_ast) {
                    eprintln!(
                        "Raw AST validation failed for {}:\n{}",
                        file_path.display(),
                        e
                    );
                    write_diagnostics_to_file(
                        parser_result.diagnostic_manager(),
                        file_path,
                        "Raw AST validation error",
                    );
                    write_ast_to_file(&raw_ast, file_path, "Raw AST validation error");
                    return None;
                }
                let diagnostic_manager = parser_result.take_diagnostic_manager();
                Some((raw_ast, diagnostic_manager))
            } else {
                eprintln!("Parsing failed (no AST) for file {}", file_path.display());
                write_diagnostics_to_file(
                    parser_result.diagnostic_manager(),
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

// Normalizes a raw AST and checks its correctness, returning the normalized AST along with the updated diagnostic manager.
///
/// This function runs the normalization process on the provided raw AST, validating the resulting normalized AST.
/// It writes diagnostic information and, in case of errors, writes the normalized AST and diagnostic details to files
/// to facilitate debugging.
///
/// # Arguments
///
/// * `raw_ast` - The raw AST to be normalized.
/// * `diagnostic_manager` - The diagnostic manager to accumulate messages during normalization.
/// * `file_path` - The path to the source file associated with the AST, used for naming output files.
///
/// # Returns
///
/// Returns `Some((Ast, DiagnosticManager))` containing the normalized AST and diagnostic manager if normalization
/// succeeds without blocking errors. Returns `None` if normalization fails or errors are found.
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
/// if let Some((normalized_ast, diag_manager)) = normalize_and_check_ast(raw_ast, diagnostic_manager, &file_path) {
///     // proceed with normalized AST
/// } else {
///     // handle normalization failure
/// }
/// ```
pub fn normalize_and_check_ast(
    raw_ast: Ast,
    diagnostic_manager: DiagnosticManager,
    file_path: &Path,
) -> Option<(Ast, DiagnosticManager)> {
    let mut normalizer = Normalizer::new();

    match normalizer.normalize_with_diagnostic_manager(raw_ast, diagnostic_manager) {
        Ok(mut normalizer_result) => {
            if let Some(normalized_ast) = normalizer_result.take_ast() {
                if let Err(e) = check_well_normalized(&normalized_ast) {
                    eprintln!(
                        "Normalized AST validation failed for {}:\n{}",
                        file_path.display(),
                        e
                    );
                    eprintln!("{}", normalized_ast.to_string_with_interner());
                    write_diagnostics_to_file(
                        normalizer_result.diagnostic_manager(),
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
                        file_path,
                        "Normalization success",
                    );
                }

                let diag_mgr = normalizer_result.take_diagnostic_manager();
                if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                    eprintln!("Diagnostics errors found for file {}", file_path.display());
                    return None;
                }

                // On retourne l'AST normalisé et le gestionnaire de diagnostics
                Some((normalized_ast, diag_mgr))
            } else {
                eprintln!(
                    "No normalized AST returned for file {}",
                    file_path.display()
                );
                write_diagnostics_to_file(
                    normalizer_result.diagnostic_manager(),
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

/// Analyzes a normalized AST using the provided diagnostic manager, performing semantic analysis,
/// and returns the symbol table along with the updated diagnostic manager.
///
/// This function runs semantic analysis via an `Analyzer`. It writes diagnostics to a file.
/// If any errors of severity `Error` are found, it also writes the AST and the symbol table to files
/// to aid debugging.
///
/// # Arguments
///
/// * `normalized_ast` - The normalized AST to be analyzed.
/// * `diagnostic_manager` - The initial diagnostic manager to use during analysis.
/// * `file_path` - The path to the source file associated with the AST, used for naming output files.
///
/// # Returns
///
/// Returns `Some((SymbolTable, DiagnosticManager))` if semantic analysis succeeds without blocking errors.
/// Returns `None` otherwise.
///
/// # Side Effects
///
/// - Writes a diagnostics file on each analysis.
/// - On semantic errors (`Severity::Error`), also writes the AST and symbol table to `.ast` and `.symtab` files.
/// - On critical failure (panic), writes the AST to a file.
///
/// # Panics
///
/// This function panics if writing to any of the output files fails.
///
/// # Example
///
/// ```rust
/// let (symbol_table, diag_manager) = analyze_ast(normalized_ast, diagnostic_manager, &file_path)
///     .e
pub fn analyze(
    mut normalized_ast: Ast,
    diagnostic_manager: DiagnosticManager,
    file_path: &Path,
) -> Option<(SemanticContext, DiagnosticManager)> {
    let mut analyzer = Analyzer::new();

    match analyzer.analyze_with_diagnostic_manager(&mut normalized_ast, diagnostic_manager) {
        Ok(mut analyzer_result) => {
            let diag_mgr = analyzer_result.diagnostic_manager();

            write_diagnostics_to_file(
                diag_mgr,
                file_path,
                "Analyzer tests: Semantic analysis result",
            );

            if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                eprintln!(
                    "Semantic analysis reported errors for file {}",
                    file_path.display()
                );

                // Save AST
                write_ast_to_file(&normalized_ast, file_path, "Analyzer error");

                // Save symbol table (non optional)
                if let Some(sem_ctx) = analyzer_result.semantic_context() {
                    let symtab = sem_ctx.symbol_table();
                    write_symbol_table_to_file(
                        symtab,
                        file_path,
                        "Analyzer error",
                        normalized_ast.interner(),
                    );
                }

                None
            } else {
                // No blocking errors: return the SemanticContext
                if let Some(sem_ctx) = analyzer_result.take_semantic_context() {
                    Some((sem_ctx, analyzer_result.take_diagnostic_manager()))
                } else {
                    eprintln!(
                        "Semantic analysis did not produce a SemanticContext for file {}",
                        file_path.display()
                    );
                    None
                }
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

            // Write AST even if analysis panics
            write_ast_to_file(&normalized_ast, file_path, "Analyzer panic");

            None
        }
    }
}

/// Parses, normalizes, and performs semantic analysis on a file.
///
/// This function processes the file at `file_path` using the specified `language`.
/// It goes through three main phases:
/// 1. Parsing and initial checking to produce a raw AST and diagnostics.
/// 2. Normalizing the AST and updating diagnostics.
/// 3. Semantic analysis producing a semantic context and updated diagnostics.
///
/// The `context` parameter is a descriptive string (e.g., "domain" or "problem")
/// used for error messages and logging.
///
/// The `success` parameter is a mutable reference to a boolean flag that will be
/// set to `false` if any step fails.
///
/// # Arguments
///
/// * `file_path` - The path to the file to analyze.
/// * `language` - The language descriptor used for parsing and analysis.
/// * `context` - A string slice describing the context (e.g., "domain", "problem").
/// * `success` - A mutable reference to a boolean that indicates overall success.
///
/// # Returns
///
/// Returns `Some((SemanticContext, DiagnosticManager))` if all steps succeed,
/// otherwise returns `None` and sets `success` to `false`.
///
/// # Errors
///
/// Prints an error message to standard error if parsing, normalization,
/// or semantic analysis fails.
pub fn analyze_file(
    file_path: &Path,
    language: &Language,
    context: &str,       // e.g., "domain" or "problem"
    success: &mut bool,  // mutable reference to update success flag
) -> Option<(SemanticContext, DiagnosticManager)> {
    let (raw_ast, diag_mgr) = match parse_and_check_ast(file_path, language) {
        Some(res) => res,
        None => {
            eprintln!("Parsing failed for {}", context);
            *success = false;
            return None;
        }
    };

    let (norm_ast, diag_mgr) = match normalize_and_check_ast(raw_ast, diag_mgr, file_path) {
        Some(res) => res,
        None => {
            eprintln!("Normalization failed for {}", context);
            *success = false;
            return None;
        }
    };

    let (semantic_ctx, diag_mgr) = match analyze(norm_ast, diag_mgr, file_path) {
        Some(res) => res,
        None => {
            eprintln!("Semantic analysis failed for {}", context);
            *success = false;
            return None;
        }
    };

    Some((semantic_ctx, diag_mgr))
}

/// Performs semantic linking between a domain and a problem semantic context.
///
/// This function attempts to link the provided `domain_ctx` and `problem_ctx`
/// semantic contexts using a `Linker`. It merges diagnostics from both contexts,
/// performs linking, and writes resulting diagnostics to a `.linking.diag` file named
/// `<problem>-<domain>.linking.diag` in the domain's parent directory.
///
/// # Parameters
///
/// - `domain_ctx`: The semantic context representing the domain.
/// - `problem_ctx`: The semantic context representing the problem.
/// - `domain_manager`: The diagnostic manager containing diagnostics from domain parsing.
/// - `problem_manager`: The diagnostic manager containing diagnostics from problem parsing.
/// - `domain_path`: Path to the domain file, used for diagnostic file naming.
/// - `problem_path`: Path to the problem file, used for diagnostic file naming.
///
/// # Returns
///
/// Returns `Some((LinkedSemanticContext, DiagnosticManager))` if linking succeeds without
/// error-level diagnostics. Returns `None` if linking fails or any error-level diagnostics
/// are reported.
///
/// # Behavior
///
/// - Always writes a `.linking.diag` file with the linking diagnostics.
/// - If semantic linking fails (e.g., due to missing context), an error diagnostic file is written.
/// - If linking produces error-level diagnostics, returns `None`.
///
/// # Side Effects
///
/// Writes diagnostic files to disk:
/// - `<problem>-<domain>.linking.diag`: Contains linking diagnostics.
/// - In case of failure: A separate error diagnostic file is written.
///
/// # Example
///
/// ```ignore
/// let domain_ctx = ...; // obtained from semantic analysis
/// let problem_ctx = ...;
/// let domain_path = Path::new("path/to/domain.hddl");
/// let problem_path = Path::new("path/to/problem.hddl");
///
/// let result = link(domain_ctx, problem_ctx, domain_manager, problem_manager, domain_path, problem_path);
/// if result.is_none() {
///     eprintln!("Linking failed or had errors.");
/// }
/// ```
pub fn link(
    domain_ctx: SemanticContext,
    problem_ctx: SemanticContext,
    domain_manager: DiagnosticManager,
    problem_manager: DiagnosticManager,
    domain_path: &Path,
    problem_path: &Path,
) -> Option<(LinkedSemanticContext, DiagnosticManager)> {
    let mut diagnostic_manager = DiagnosticManager::new();
    diagnostic_manager.add_diagnostic_from(domain_manager);
    diagnostic_manager.add_diagnostic_from(problem_manager);

    let mut linker = Linker::new();

    // Call link_with_diagnostic_manager and convert Result to Option
    let mut linker_result = linker
        .link_with_diagnostic_manager(domain_ctx, problem_ctx, diagnostic_manager)
        .ok()?; // convert Result to Option

    match linker_result.take_linked_semantic_context() {
        Some(linked_context) => {
            // Always write the linking diagnostics file
            write_linking_diag_to_file(
                &linker_result.take_diagnostic_manager(),
                domain_path,
                problem_path,
                "Linking tests",
            );

            let diagnostic_manager = linker_result.take_diagnostic_manager();

            if diagnostic_manager.has_diagnostics_of_severity(Severity::Error) {
                eprintln!("Linking reported errors for file {}", problem_path.display());
                None
            } else {
                Some((linked_context, diagnostic_manager))
            }
        }
        None => {
            // Linking failed due to missing semantic context
            eprintln!(
                "Linking failed for domain {} and problem {}: missing context",
                domain_path.display(),
                problem_path.display()
            );

            write_error_diagnostic_file_for_domain_and_problem(
                domain_path,
                problem_path,
                "Linking error",
                "Missing semantic context",
            );

            None
        }
    }
}
