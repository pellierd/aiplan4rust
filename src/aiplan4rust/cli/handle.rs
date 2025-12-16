use std::fs;
use std::path::{Path, PathBuf};
use colored::Colorize;
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeSerializable};

/// Saves the parsed semantic context to a file and prints the output file location.
///
/// This function attempts to serialize the provided semantic context to the specified
/// output file using the given serialization format. After successfully writing the file,
/// it prints a message showing the absolute path of the output file.
///
/// # Arguments
/// * `context` - The parsed semantic context implementing `SerdeSerializable`.
/// * `format` - The serialization format to use (e.g., JSON, YAML, TOML).
/// * `output` - Path to the output file where the serialized content should be written.
///
/// # Behavior
/// - If serialization fails, an error message is printed and the function returns early.
/// - If successful, the absolute path of the output file is printed to the console.
///
/// # Example
/// ```rust
/// let context = get_semantic_context(); // returns an impl SerdeSerializable
/// let output_file = "domain.json";
/// save_output_file(context, SerdeFormat::Json, output_file);
/// // Output: "===> Output file produced (/absolute/path/to/domain.json)"
/// ```
pub fn save_output_file(context: &impl SerdeSerializable, format: SerdeFormat, output: &str) {
    if let Err(e) = context.serialize_to_file(format, output) {
        eprintln!("Error saving file: {}", e);
        return;
    }

    let absolute_output = Path::new(output)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| output.to_string());

    println!(
        "{} Output file produced ({})",
        "===> ".blue().bold(),
        absolute_output
    );
}

/// Ensures that the parent directory of a given file path exists.
/// Exits the process on error.
pub fn ensure_parent_dir_exists(path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Error creating output directory {}: {}", parent.display(), e);
                std::process::exit(1);
            }
        }
    }
}

/// Ensures that the given directory exists.
/// Exits the process on error.
pub fn ensure_dir_exists(dir: &str) {
    if !Path::new(dir).exists() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("Error creating directory {}: {}", dir, e);
            std::process::exit(1);
        }
    }
}

/// Generates a default output filename from domain and optionally problem files.
///
/// The generated filename will be:
/// - If `problem_file` is `Some`: `{domain_stem}-{problem_stem}.{format}`
/// - If `problem_file` is `None`: `{domain_stem}.{format}`
///
/// If `out_dir` is provided, the filename is joined to that directory.
///
/// # Arguments
///
/// * `domain_file` - Path to the domain file.
/// * `problem_file` - Optional path to the problem file.
/// * `format` - Desired serialization format for the output.
/// * `out_dir` - Optional output directory.
///
/// # Errors
///
/// Returns a `CliError` if either input file has an invalid filename.
///
/// # Examples
///
/// ```rust
/// let output = generate_default_output_filename("domain.pddl", Some("problem.pddl"), SerdeFormat::Json, None)?;
/// let output_single = generate_default_output_filename("domain.pddl", None, SerdeFormat::Json, None)?;
/// ```
pub fn generate_default_output_filename(
    domain_file: &str,
    problem_file: Option<&str>,
    format: SerdeFormat,
    out_dir: Option<&str>,
) -> Result<String, CliError> {
    // Extract domain stem
    let domain_stem = Path::new(domain_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CliError::invalid_file_name(domain_file))?;

    // Build filename depending on optional problem file
    let filename = match problem_file {
        Some(problem) => {
            let problem_stem = Path::new(problem)
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| CliError::invalid_file_name(problem))?;
            format!("{}-{}.{}", domain_stem, problem_stem, format)
        }
        None => format!("{}.{}", domain_stem, format),
    };

    // Join with output directory if provided
    let path = match out_dir {
        Some(dir) => Path::new(dir).join(filename),
        None => PathBuf::from(filename),
    };

    Ok(path.to_string_lossy().into_owned())
}
