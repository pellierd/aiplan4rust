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
/// * `diagnostic_manager` - The diagnostic manager to render.
/// * `file_path` - The path of the file for which diagnostics are produced.
///
/// # Panics
/// Panics if the `.diag` file cannot be created or written.
pub fn write_diagnostics_to_file(diagnostic_manager: &DiagnosticManager, file_path: &Path) {
    // Build the target .diag path
    let diag_path = file_path.with_extension("diag");

    // Create the file
    let mut diag_file = File::create(&diag_path)
        .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));

    // Prepare a prominent header
    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                                DIAGNOSTIC REPORT                             *\n\
         ********************************************************************************\n\
         Generated for file: {}\n\
         Generated at UTC:  {}\n\
         ********************************************************************************\n\n",
        file_path.display(),
        timestamp.to_rfc3339(),
    );

    diag_file.write_all(header.as_bytes()).unwrap_or_else(|_| {
        panic!(
            "Failed to write header to diag file: {}",
            diag_path.display()
        )
    });

    // Render diagnostics into a buffer
    let mut buffer = Vec::new();
    Renderer::write_to(diagnostic_manager, &mut buffer, false)
        .expect("Failed to write diagnostics");

    // Write buffer contents into the file
    diag_file.write_all(&buffer).unwrap_or_else(|_| {
        panic!(
            "Failed to write diagnostics to diag file: {}",
            diag_path.display()
        )
    });
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
pub fn write_ast_to_file(ast: &Ast, file_path: &Path) {
    let ast_path = file_path.with_extension("ast");

    let mut ast_file = File::create(&ast_path)
        .unwrap_or_else(|_| panic!("Failed to create ast file: {}", ast_path.display()));

    let timestamp = Utc::now();
    let header = format!(
        "********************************************************************************\n\
         *                                 AST OUTPUT                                   *\n\
         ********************************************************************************\n\
         Generated for file: {}\n\
         Generated at UTC:  {}\n\
         ********************************************************************************\n\n",
        file_path.display(),
        timestamp.to_rfc3339(),
    );

    ast_file
        .write_all(header.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write header to ast file: {}", ast_path.display()));

    let ast_string = ast.to_string_with_interner();

    ast_file
        .write_all(ast_string.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to write AST to ast file: {}", ast_path.display()));
}
