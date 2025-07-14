use std::{fs, io};
use std::io::Read;
use std::path::{Path, PathBuf};

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
        return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist"));
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
            if SUPPORTED_EXTENSIONS.iter().any(|&s| s.eq_ignore_ascii_case(ext)) {
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
    try_collect_domain_files(domain_dir)
        .unwrap_or_else(|e| panic!("Failed to collect domain files in {}: {}", domain_dir.display(), e))
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
    files.iter()
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
