use crate::aiplan4rust::cli::io::artefact::Extension;
use std::io;
use std::path::{Path, PathBuf};

/// Generates a debug "parsed" output path for a given input file.
///
/// This function simplifies calling `default_output_path` for the case
/// of a single input file and uses `Extension::Parsed` by debug.
///
/// # Arguments
/// * `input_path` - The path to the input file.
/// * `out_dir` - The output directory where the file should be placed.
///
/// # Errors
/// Returns `CliError::InvalidFileName` if the input file stem is missing or not valid UTF-8.
pub fn default_parsed_output_path(input_path: &Path, out_dir: &Path) -> Result<PathBuf, io::Error> {
    default_output_path(input_path, None, Some(Extension::Parsed), Some(out_dir))
}

/// Generates an output path for a parsed file using the input file’s extension.
///
/// This function is similar to `default_parsed_output_path`, but it does not
/// force the `Extension::Parsed` suffix. Instead, it lets `default_output_path`
/// determine the extension based on the input path.
///
/// # Arguments
/// * `input_path` - The path to the input file.
/// * `out_dir` - The output directory where the file should be placed.
///
/// # Errors
/// Returns `CliError::InvalidFileName` if the input file stem is missing or not valid UTF-8.
pub fn output_path(input_path: &Path, out_dir: &Path) -> Result<PathBuf, io::Error> {
    default_output_path(input_path, None, None, Some(out_dir))
}

/// Generates a debug "lifted" output path for a given domain and problem file.
///
/// This function simplifies calling `default_output_path` for the case
/// of linking a domain with a single problem file and uses `Extension::Lifted` by debug.
///
/// # Arguments
/// * `domain_path` - The path to the domain file.
/// * `problem_path` - The path to the problem file.
/// * `out_dir` - The output directory where the file should be placed.
///
/// # Errors
/// Returns `io::Error` if the file stem is missing or invalid UTF-8.
pub fn default_lifted_output_path(
    domain_path: &Path,
    problem_path: &Path,
    out_dir: &Path,
) -> Result<PathBuf, io::Error> {
    default_output_path(
        domain_path,
        Some(problem_path),
        Some(Extension::Lifted),
        Some(out_dir),
    )
}

/// Generates a debug "grounded" output path for a given domain and problem file.
///
/// This function simplifies calling `default_output_path` for the case
/// of grounding a domain with a single problem file and uses `Extension::Grounded` by debug.
///
/// # Arguments
/// * `domain_path` - The path to the domain file.
/// * `problem_path` - The path to the problem file.
/// * `out_dir` - The output directory where the file should be placed.
///
/// # Errors
/// Returns `io::Error` if the file stem is missing or invalid UTF-8.
pub fn default_grounded_output_path(
    domain_path: &Path,
    problem_path: &Path,
    out_dir: &Path,
) -> Result<PathBuf, io::Error> {
    default_output_path(
        domain_path,
        Some(problem_path),
        Some(Extension::Grounded),
        Some(out_dir),
    )
}

/// Generates a debug output path from a domain file and optionally a problem file,
/// using the specified output `Extension`, and optionally placing the result in an
/// output directory.
///
/// # Behavior
///
/// The generated filename follows these rules:
///
/// - If `problem_path` is `Some`:
///   `{domain_stem}-{problem_stem}.{extension}`
/// - If `problem_path` is `None`:
///   `{domain_stem}.{extension}`
///
/// If `out_dir` is provided, the generated filename is joined to that directory.
/// Otherwise, the path is relative to the current working directory.
///
/// # Arguments
///
/// * `domain_path` - Path to the domain file (e.g., `domain.pddl`).
/// * `problem_path` - Optional path to the problem file (e.g., `problem.pddl`).
/// * `extension` - The desired output file extension.
/// * `out_dir` - Optional output directory.
///
/// # Errors
///
/// Returns `std::io::Error` if:
/// - The domain file has no valid file stem,
/// - The problem file is provided but has no valid file stem,
/// - Or either filename is not valid UTF-8.
///
/// # Examples
///
/// ```rust
/// use std::path::{Path, PathBuf};
/// use crate::aiplan4rust::artefact::{default_output_path, Extension};
///
/// let domain = Path::new("domain.pddl");
/// let problem = Path::new("problem.pddl");
///
/// let output = default_output_path(
///     domain,
///     Some(problem),
///     Extension::Parsed,
///     None,
/// ).unwrap();
///
/// assert_eq!(output, PathBuf::from("domain-problem.prs"));
/// ```
///
/// ```rust
/// use std::path::{Path, PathBuf};
/// use crate::aiplan4rust::artefact::{default_output_path, Extension};
///
/// let domain = Path::new("domain.pddl");
/// let out_dir = Path::new("out_dir");
///
/// let output = default_output_path(
///     domain,
///     None,
///     Extension::Parsed,
///     Some(out_dir),
/// ).unwrap();
///
/// assert_eq!(output, PathBuf::from("out_dir/domain.prs"));
/// ```
pub fn default_output_path(
    domain_path: &Path,
    problem_path: Option<&Path>,
    extension: Option<Extension>,
    out_dir: Option<&Path>,
) -> Result<PathBuf, io::Error> {
    // Extract domain stem
    let domain_stem = file_stem_or_error(domain_path)?;

    // Determine the extension to use
    let ext_str = match extension {
        Some(ext) => ext.to_string(), // assume Extension implements Display or ToString
        None => domain_path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default(), // no extension
    };

    // Build filename
    let filename = if let Some(problem) = problem_path {
        let problem_stem = file_stem_or_error(problem)?;
        if ext_str.is_empty() {
            format!("{}-{}", domain_stem, problem_stem)
        } else {
            format!("{}-{}.{}", domain_stem, problem_stem, ext_str)
        }
    } else {
        if ext_str.is_empty() {
            domain_stem.to_string()
        } else {
            format!("{}.{}", domain_stem, ext_str)
        }
    };

    // Join with output directory if provided
    Ok(out_dir.unwrap_or_else(|| Path::new("")).join(filename))
}

/// Extracts the file stem (file name without extension) from a given `Path`.
///
/// # Arguments
///
/// * `path` - A reference to a `Path` from which to extract the file stem.
///
/// # Returns
///
/// * `Ok(&str)` - The file stem as a string slice if it exists and is valid UTF-8.
/// * `Err(io::Error)` - If the path has no file stem or contains invalid UTF-8.
///
/// # Example
///
/// ```rust
/// use std::path::Path;
///
/// let path = Path::new("domain.pddl");
/// let stem = file_stem_or_error(path).unwrap();
/// assert_eq!(stem, "domain");
/// ```
fn file_stem_or_error(path: &Path) -> Result<&str, io::Error> {
    let stem = path.file_stem().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("No file stem found for path {}", path.display()),
        )
    })?;

    let stem_str = stem.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid UTF-8 in path {}", path.display()),
        )
    })?;

    Ok(stem_str)
}
