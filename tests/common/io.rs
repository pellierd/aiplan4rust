#![allow(dead_code)]

use aiplan4rust::aiplan4rust::diagnostic::DiagnosticManager;
use aiplan4rust::aiplan4rust::syntax::ast::Ast;
use aiplan4rust::Renderer;
use chrono::Utc;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};
use walkdir::WalkDir;
use aiplan4rust::aiplan4rust::interner::{InternerDisplay, StringInterner};
use aiplan4rust::aiplan4rust::semantic::SymbolTable;

/// Supported file extensions (case-insensitive).
const SUPPORTED_EXTENSIONS: &[&str] = &["pddl", "hddl"];

/// Prefix that identifies problem files by their filename.
const PROBLEM_PREFIX: &str = "pb";

/// Substring that, if present in a filename, excludes the file from being considered a problem file.
const DOMAIN_SUFFIX_EXCLUSION: &str = "-domain";

/// Reads the file at `path`, returning a `Result<String>` if successful or an `io::Error` if not.
///
/// # Arguments
/// * `path` - Path to the file.
///
/// # Returns
/// The file contents as a `String`.
pub fn try_read_file(path: &Path) -> io::Result<String> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "File does not exist",
        ));
    }

    let mut source = String::new();
    let mut file = fs::File::open(path)?;
    file.read_to_string(&mut source)?;
    Ok(source)
}

/// Reads the file at `path`, panicking if any error occurs.
///
/// # Panics
/// Panics if the file cannot be read.
///
/// # Arguments
/// * `path` - Path to the file.
///
/// # Returns
/// The file contents as a `String`.
pub fn read_file(path: &Path) -> String {
    try_read_file(path).unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e))
}

/// Collects all supported files from the specified directory.
///
/// # Arguments
/// * `domain_dir` - A reference to the directory to search.
///
/// # Returns
/// A `Result` containing a vector of `PathBuf` if successful, or an `io::Error`.
pub fn try_collect_domain_files(domain_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(domain_dir)?;

    let mut files = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if SUPPORTED_EXTENSIONS
                .iter()
                .any(|&s| s.eq_ignore_ascii_case(ext))
            {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Same as `try_collect_domain_files`, but panics on error.
///
/// # Panics
/// Panics if any I/O error occurs.
///
/// # Arguments
/// * `domain_dir` - A reference to the directory to search.
///
/// # Returns
/// A vector of `PathBuf`.
pub fn collect_domain_files(domain_dir: &Path) -> Vec<PathBuf> {
    try_collect_domain_files(domain_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to collect domain files in {}: {}",
            domain_dir.display(),
            e
        )
    })
}

/// Filters a list of file paths to return only problem files.
///
/// A problem file is defined as a file whose name starts with `"pb"`
/// and does **not** contain the substring `"-domain"` in its filename.
///
/// # Arguments
///
/// * `files` - A slice of `PathBuf` representing the list of files to filter.
///
/// # Returns
///
/// A `Vec<PathBuf>` containing only the problem files matching the criteria.
///
/// # Examples
///
/// ```
/// let files = vec![
///     PathBuf::from("pb001.hddl"),
///     PathBuf::from("pb002-domain.hddl"),
///     PathBuf::from("somefile.hddl"),
/// ];
/// let problems = filter_problem_files(&files);
/// assert_eq!(problems, vec![PathBuf::from("pb001.hddl")]);
/// ```
pub fn filter_problem_files(files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with(PROBLEM_PREFIX) && !f.contains(DOMAIN_SUFFIX_EXCLUSION))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// Extracts the stem (file name without extension) from a given path as a `String`.
///
/// # Panics
///
/// Panics if the path does not have a file stem.
pub fn get_file_stem_as_string(path: &Path) -> String {
    path.file_stem()
        .expect("Path has no file stem")
        .to_string_lossy()
        .to_string()
}

/// Extracts the stem (file name without extension) from a given path as a `String`.
///
/// Returns `None` if the path has no file stem.
pub fn try_get_file_stem_as_string(path: &Path) -> Option<String> {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
}

/// Recursively deletes all files with the given extension under the specified directory.
///
/// # Arguments
/// * `root_dir` - The root directory to start the search.
/// * `extension` - The extension to match (e.g., "diag").
pub fn delete_all_files_with_extension(root_dir: &Path, extension: &str) {
    for entry in WalkDir::new(root_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == extension {
                match fs::remove_file(path) {
                    Ok(_) => println!("Deleted: {}", path.display()),
                    Err(e) => eprintln!("Failed to delete {}: {}", path.display(), e),
                }
            }
        }
    }
}

/// Writes the diagnostics to a `.diag` file next to the given path,
/// including a prominent header indicating which test produced it and the timestamp.
///
/// # Arguments
/// * `diagnostic_manager` - The diagnostic manager to renderers.
/// * `file_path` - The path of the file for which diagnostics are produced.
///
/// # Panics
/// Panics if the `.diag` file cannot be created or written.
pub fn write_diagnostics_to_file(
    diagnostic_manager: &DiagnosticManager,
    interner: &StringInterner,
    file_path: &Path,
    context: &str,
) {
    let diag_path = file_path.with_extension("diag");

    let mut diag_file = File::create(&diag_path)
        .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));

    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                           DIAGNOSTIC REPORT                                  *\n\
         ********************************************************************************\n\
         Context: {}\n\
         File:    {}\n\
         UTC:     {}\n\
         ********************************************************************************\n\n",
        context,
        file_path.display(),
        timestamp.to_rfc3339(),
    );

    diag_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write header to diag file: {}", diag_path.display()));

    let mut buffer = Vec::new();
    Renderer::write_to(diagnostic_manager, interner, &mut buffer, false)
        .expect("Failed to write diagnostics");

    diag_file
        .write_all(&buffer)
        .unwrap_or_else(|_| panic!("Failed to write diagnostics to diag file: {}", diag_path.display()));
}

/// Writes the string representation of an AST to a `.ast` file next to the given path,
/// including a prominent header with file info and timestamp.
///
/// # Arguments
/// * `ast` - The AST to serialize as a string.
/// * `file_path` - The original file path (the `.ast` file will be created with the same base name).
/// * `interner` - The interner needed for `to_string_with_interner`.
///
/// # Panics
/// Panics if the `.ast` file cannot be created or written.
pub fn write_ast_to_file(ast: &Ast, file_path: &Path, context: &str) {
    let ast_path = file_path.with_extension("ast");

    let mut ast_file = File::create(&ast_path)
        .unwrap_or_else(|_| panic!("Failed to create AST file: {}", ast_path.display()));

    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                                AST DUMP                                      *\n\
         ********************************************************************************\n\
         Context: {}\n\
         File:    {}\n\
         UTC:     {}\n\
         ********************************************************************************\n\n",
        context,
        file_path.display(),
        timestamp.to_rfc3339(),
    );

    ast_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write header to AST file: {}", ast_path.display()));

    let ast_str = ast.to_string_with_interner();
    ast_file
        .write_all(ast_str.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write AST to file: {}", ast_path.display()));
}

/// Writes a `.diag` file manually for fatal errors such as parsing or normalization failures.
///
/// This function is intended to handle critical errors where structured diagnostics (e.g., via a
/// `DiagnosticManager`) may not be available—such as when the parser or simplify crashes early
/// and cannot return diagnostics in the usual format.
///
/// It creates a `.diag` file next to the original input file, containing:
/// - A clear header labeled "DIAGNOSTIC REPORT"
/// - The context (e.g., "Parsing error", "Normalization error")
/// - The file path
/// - A UTC timestamp
/// - The error message itself
///
/// # Arguments
///
/// * `file_path` - The path to the original file that failed to process (used to derive `.diag` filename).
/// * `context` - A short string describing the phase or nature of the error (e.g., "Parsing error").
/// * `error_message` - A description of the error to be included in the report.
///
/// # Panics
///
/// Panics if the `.diag` file cannot be created or written to (e.g., due to file system errors).
///
/// # Example
///
/// ```rust
/// use std::path::Path;
///
/// write_error_diagnostic_file(
///     Path::new("examples/domain.pddl"),
///     "Parsing error",
///     "Unexpected token 'define' at line 1",
/// );
/// ```
///
/// This will create a file `examples/domain.diag` containing the error message and diagnostic context.
pub fn write_error_diagnostic_file(file_path: &Path, context: &str, error_message: &str) {
    let diag_path = file_path.with_extension("diag");
    let mut diag_file = File::create(&diag_path)
        .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));
    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                           DIAGNOSTIC REPORT                                  *\n\
         ********************************************************************************\n\
         Context: {}\n\
         File:    {}\n\
         UTC:     {}\n\
         ********************************************************************************\n\n\
         Error: {}\n",
        context,
        file_path.display(),
        timestamp.to_rfc3339(),
        error_message
    );
    diag_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write diagnostic file: {}", diag_path.display()));
}

pub fn write_error_diagnostic_file_for_domain_and_problem(
    domain_path: &Path,
    problem_path: &Path,
    context: &str,
    error_message: &str,
) {
    // Construction du nom du fichier diag : <problem>-<domain>.linking.diag
    let problem_stem = problem_path.file_stem().unwrap_or_default();
    let domain_stem = domain_path.file_stem().unwrap_or_default();

    let diag_file_name = format!(
        "{}-{}.linking.diag",
        problem_stem.to_string_lossy(),
        domain_stem.to_string_lossy()
    );

    // Répertoire cible (ici on met le fichier à côté du fichier problème)
    let diag_path = problem_path.parent().unwrap_or_else(|| Path::new(".")).join(diag_file_name);

    let mut diag_file = File::create(&diag_path)
        .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));

    let timestamp = Utc::now();

    let header = format!(
        "********************************************************************************\n\
         *                           DIAGNOSTIC REPORT                                  *\n\
         ********************************************************************************\n\
         Context: {}\n\
         Domain File:    {}\n\
         Problem File:   {}\n\
         UTC:           {}\n\
         ********************************************************************************\n\n\
         Error: {}\n",
        context,
        domain_path.display(),
        problem_path.display(),
        timestamp.to_rfc3339(),
        error_message
    );

    diag_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write diagnostic file: {}", diag_path.display()));
}

/// Dumps the contents of a `SymbolTable` to a `.symtab` file adjacent to the given source file.
///
/// This function writes a human-readable representation of the symbol table, including a header
/// with context information and a UTC timestamp. The file will have the same base name as the input
/// `file_path`, but with a `.symtab` extension.
///
/// # Arguments
///
/// * `symbol_table` - A reference to the `SymbolTable` to be dumped.
/// * `file_path` - The path to the source file for which the symbol table is associated.
/// * `context` - A string describing the context (e.g., "Analyzer error", "Test run").
/// * `interner` - A reference to the `Interner` used to resolve symbol names.
///
/// # Panics
///
/// This function will panic if:
/// - The file cannot be created.
/// - The header or symbol table contents cannot be written.
///
/// # Example
///
/// ```rust
/// let symbol_table = SymbolTable::new();
/// let interner = Interner::new();
/// let path = Path::new("tests/example/input_file");
/// write_symbol_table_to_file(&symbol_table, &path, "Unit test symbol table dump", &interner);
/// ```
///
pub fn write_symbol_table_to_file(
    symbol_table: &SymbolTable,
    file_path: &Path,
    context: &str,
    interner: &StringInterner,
) {
    let symtab_path = file_path.with_extension("symtab");

    let mut symtab_file = File::create(&symtab_path)
        .unwrap_or_else(|_| panic!("Failed to create symbol table file: {}", symtab_path.display()));

    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                             SYMBOL TABLE DUMP                                *\n\
         ********************************************************************************\n\
         Context: {}\n\
         File:    {}\n\
         UTC:     {}\n\
         ********************************************************************************\n\n",
        context,
        file_path.display(),
        timestamp.to_rfc3339(),
    );

    symtab_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write header to symbol table file: {}", symtab_path.display()));

    let symtab_str = symbol_table.to_string_with_interner(interner);
    symtab_file
        .write_all(symtab_str.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write symbol table to file: {}", symtab_path.display()));
}

/// Writes linking diagnostics to a `.linking.diag` file next to the given problem file.
///
/// This function collects diagnostics from the [`DiagnosticManager`] and writes them to a
/// human-readable text file, including metadata such as file paths, context string, and
/// current UTC timestamp. The output file is named after the problem file with the suffix
/// `.linking.diag`.
///
/// # Parameters
///
/// - `diagnostic_manager`: A reference to the [`DiagnosticManager`] holding the diagnostics to output.
/// - `interner`: A reference to the [`StringInterner`] used to resolve symbol identifiers for readable output.
/// - `domain_path`: Path to the domain PDDL file (for display purposes only).
/// - `problem_path`: Path to the problem PDDL file. The output file will be written in the same directory,
///   using the problem file's stem followed by `.linking.diag`.
/// - `context`: A user-defined label or description for the current operation or invocation context.
///
/// # Panics
///
/// This function will panic if:
///
/// - The diagnostic output file cannot be created.
/// - Writing the header or diagnostics to the file fails.
///
/// # Output
///
/// A file named `<problem_stem>.linking.diag` will be created in the same directory as the
/// problem file. This file contains:
/// - A structured header (with context, file paths, and timestamp),
/// - Followed by the rendered diagnostics.
///
/// # Example
///
/// ```no_run
/// write_linking_diag_to_file(
///     &diagnostic_manager,
///     &interner,
///     Path::new("domain.pddl"),
///     Path::new("problem.pddl"),
///     "Linking phase",
/// );
/// // Creates a file like `problem.linking.diag` with diagnostic output.
/// ```
///
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
/// [`StringInterner`]: crate::symbols::StringInterner
pub fn write_linking_diag_to_file(
    diagnostic_manager: &DiagnosticManager,
    interner: &StringInterner,
    domain_path: &Path,
    problem_path: &Path,
    context: &str,
) {
    // Build a filename like "<problem>.linking.diag"
    let problem_stem = problem_path.file_stem().unwrap_or_default();

    let file_name = format!(
        "{}.linking.diag",
        problem_stem.to_string_lossy()
    );

    let diag_path = problem_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name);

    let mut diag_file = File::create(&diag_path)
        .unwrap_or_else(|_| panic!("Failed to create linking diagnostic file: {}", diag_path.display()));

    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                             LINKING DIAGNOSTICS                              *\n\
         ********************************************************************************\n\
         Context: {}\n\
         Domain file:  {}\n\
         Problem file: {}\n\
         UTC:         {}\n\
         ********************************************************************************\n\n",
        context,
        domain_path.display(),
        problem_path.display(),
        timestamp.to_rfc3339(),
    );

    diag_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write header to linking diagnostic file: {}", diag_path.display()));

    // Convert diagnostics to a string using the Renderer
    let mut buffer = Vec::new();
    Renderer::write_to(diagnostic_manager, interner, &mut buffer, false)
        .expect("Failed to write diagnostics");

    diag_file
        .write_all(&buffer)
        .unwrap_or_else(|_| panic!("Failed to write linking diagnostics to file: {}", diag_path.display()));
}
