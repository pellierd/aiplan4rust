use std::fs;
use std::path::{Path, PathBuf};
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::{SerdeExtension, SerdeFormat, SerdeSerializable};

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

/// Generates the output filename based on input file, format, and optional output directory.
pub fn generate_parsed_filename(
    input_file: &str,
    format: SerdeFormat,
    out_dir: Option<&str>,
) -> String {
    let base_name = Path::new(input_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(input_file);
    let extension = SerdeExtension::from(format).as_str();

    match out_dir {
        Some(dir) => {
            let mut path = PathBuf::from(dir);
            path.push(format!("{}.{}", base_name, extension));
            path.to_string_lossy().into_owned()
        }
        None => format!("{}.{}", base_name, extension),
    }
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
