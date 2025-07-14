use crate::common::io::{
    read_file, write_ast_to_file, write_diagnostics_to_file, write_error_diagnostic_file,
    write_symbol_table_to_file,
};
use aiplan4rust::aiplan4rust::diagnostic::DiagnosticManager;
use aiplan4rust::aiplan4rust::semantic::SymbolTable;
use aiplan4rust::aiplan4rust::syntax::ast::Ast;
use aiplan4rust::aiplan4rust::validation::normalization::check_well_normalized;
use aiplan4rust::aiplan4rust::Analyzer;
use aiplan4rust::{check_well_formed, Language, Normalizer, Parser, Severity};
use std::path::Path;

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
/// Returns `None` if parsing fails, no AST is produced, or if the AST fails the well-formedness check.
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
pub fn analyze_ast(
    mut normalized_ast: Ast,
    diagnostic_manager: DiagnosticManager,
    file_path: &Path,
) -> Option<(SymbolTable, DiagnosticManager)> {
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
                let symtab = analyzer_result.semantic_context()?.symbol_table();
                write_symbol_table_to_file(
                    symtab,
                    file_path,
                    "Analyzer error",
                    normalized_ast.interner(),
                );

                None
            } else {
                // No blocking errors: return symbol table
                let symbol_table = analyzer_result.semantic_context_mut()?.take_symbol_table();
                Some((symbol_table, analyzer_result.take_diagnostic_manager()))
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
