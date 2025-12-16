use clap::error::ErrorKind;
use clap::ArgMatches;
use colored::Colorize;
use std::path::Path;
use std::time::Instant;

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::cli::handle::{ensure_dir_exists, ensure_parent_dir_exists, generate_default_output_filename, save_output_file};
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::error::CliError;

/// Handles the logic for the `parse` CLI subcommand.
///
/// This function performs the following steps:
/// 1. Validates the provided CLI arguments using `validate_parse_args`.
/// 2. Retrieves the domain and/or problem files specified by the user.
/// 3. Determines whether a single input file or multiple files are provided.
/// 4. Generates appropriate output filenames if not explicitly provided.
/// 5. Ensures that all necessary output directories exist.
/// 6. Calls the `parse` function for each input file.
///
/// # Behavior
/// - **Single input file**:
///     - Uses `-o/--output` if provided.
///     - If no output file is specified, a default filename is generated based on the input file and serialization format.
/// - **Multiple input files**:
///     - `-o/--output` is forbidden (validation ensures this).
///     - Requires `-d/--out-dir` to specify the output directory; defaults to the current directory if not provided.
///     - Generates output filenames inside the specified output directory for each input file.
///
/// # Arguments
/// * `matches` - A reference to `ArgMatches` containing the parsed CLI arguments for the `parse` subcommand.
///
/// # Panics
/// - The function will terminate the process (`std::process::exit(1)`) if argument validation fails.
/// - The function may also exit if it fails to create required directories or write output files.
///
/// # Example
/// ```rust
/// let matches = build_parse_subcommand().get_matches();
/// handle_parse_command(&matches);
/// ```
pub fn handle_parse_command(matches: &ArgMatches) -> Result<(), CliError> {
    // Validate CLI arguments first; exit on error
    if let Err(err) = validate_parse_args(matches) {
        if let Err(print_err) = err.print() {
            eprintln!("Failed to print clap error: {}", print_err);
        }
        std::process::exit(1);
    }

    // Collect the input files from the CLI
    let files: Vec<String> = match matches.get_many::<String>(FILES_ARG) {
        Some(values) => values.cloned().collect(),
        None => {
            eprintln!("Error: no input files provided.");
            std::process::exit(1);
        }
    };

    // Retrieve the output serialization format (default: JSON)
    let format = *matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();

    match files.len() {
        1 => {
            // Single input file
            let input_file = &files[0];
            let output = match matches.get_one::<String>(OUTPUT_ARG) {
                Some(s) => s.clone(),
                None => generate_default_output_filename(input_file, None, format, None)?,
            };


            // Ensure parent directory exists
            ensure_parent_dir_exists(&output);

            // Parse and write the output file
            parse_from_files(input_file, format, &output);
            Ok(())
        }
        _ => {
            // Multiple input files
            let out_dir = matches
                .get_one::<String>(OUT_DIR_ARG)
                .cloned()
                .unwrap_or_else(|| CURRENT_DIR.to_string()); // default to current directory

            // Ensure the output directory exists
            ensure_dir_exists(&out_dir);

            for input_file in &files {
                let output_path = generate_default_output_filename(input_file, None, format, Some(&out_dir))?;

                // Ensure parent directories exist for each output file
                ensure_parent_dir_exists(&output_path);

                // Parse and write each output file
                parse_from_files(input_file, format, &output_path);
            }
            Ok(())
        }
    }
}

/// Parses a single input PDDL/HDDL file and optionally writes the serialized output to a file.
///
/// This function performs the following steps:
/// 1. Displays a parsing start message including the absolute path of the input file.
/// 2. Parses the input file using the `Frontend`.
/// 3. Renders any diagnostics (errors and warnings) to the console.
/// 4. Displays the parsing result including the number of errors, warnings, and elapsed time.
/// 5. If there are no errors, serializes the semantic context to the specified output file.
///
/// # Arguments
/// * `input_file` - Path to the input PDDL/HDDL file to parse.
/// * `format` - The serialization format for the output file (e.g., JSON, YAML, TOML).
/// * `output` - Path to the output file where the serialized content will be written.
///
/// # Behavior
/// - If parsing fails, the error is printed and the function returns early.
/// - If parsing succeeds but there are errors in the input file, no output file is produced.
/// - If parsing succeeds with no errors, the semantic context is serialized to the output file.
///
/// # Example
/// ```rust
/// let input_file = "domain.pddl";
/// let output_file = "domain.json";
/// parse(input_file, SerdeFormat::Json, output_file);
/// ```
fn parse_from_files(input_file: &str, format: SerdeFormat, output: &str) {
    let start_time = Instant::now();

    // Display parsing start message and get absolute path
    display_parsing_start(input_file);

    // Perform parsing
    let frontend = Frontend::new();
    let result = match frontend.parse_file(input_file) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", e);
            return; // Early return on parsing error
        }
    };

    // Render diagnostics
    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display().unwrap_or_else(|e| {
        eprintln!("Warning: failed to render diagnostics: {}", e);
    });

    // Count errors and warnings
    let error_count = result
        .diagnostic_manager()
        .count_diagnostics_of_severity(Severity::Error);
    let warning_count = result
        .diagnostic_manager()
        .count_diagnostics_of_severity(Severity::Warning);
    let elapsed_time = start_time.elapsed().as_secs_f32();

    // Display parsing result summary
    display_parsing_result(error_count, warning_count, elapsed_time);

    // Early return if errors exist
    if error_count > 0 {
        println!(
            "{} No output file produced due to errors.",
            "===> ".blue().bold()
        );
        return;
    }

    // Serialize semantic context to output file
    if let Some(context) = result.semantic_context() {
        save_output_file(context, format, output);
    }
}

/// Displays a parsing start message for a given input file and returns its absolute path.
///
/// This function performs the following steps:
/// 1. Resolves the absolute (canonical) path of the input file.
///    - If canonicalization fails, the original path is returned.
/// 2. Prints a formatted message indicating that parsing has started, including:
///    - The tool name (`aiplan4rust`)
///    - The tool version, automatically obtained from Cargo.toml
///    - The input file path
///
/// # Arguments
/// * `input_file` - Path to the input PDDL/HDDL file to parse.
///
/// # Returns
/// * `String` - The absolute path of the input file, or the original path if canonicalization fails.
///
/// # Example
/// ```rust
/// let input_file = "domain.pddl";
/// let absolute_path = display_parsing_start(input_file);
/// println!("Parsing started for: {}", absolute_path);
/// ```
fn display_parsing_start(input_file: &str) -> String {
    // Resolve the absolute path; fallback to the original path if canonicalization fails
    let input_path = Path::new(input_file)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| input_file.to_string());

    // Use the version from Cargo.toml automatically
    let version = env!("CARGO_PKG_VERSION");

    // Print the parsing start message
    println!(
        "\n{:>10} aiplan4rust v{} ({})",
        "Parsing".green().bold(),
        version,
        input_path
    );

    input_path
}

/// Displays a summary of the parsing results, including the number of errors,
/// warnings, and the total elapsed time for parsing.
///
/// This function prints a formatted message to the console in the following format:
/// `"Finished <error_count> error(s), <warning_count> warning(s) in <elapsed_time>s"`.
///
/// # Arguments
/// * `error_count` - The number of errors encountered during parsing.
/// * `warning_count` - The number of warnings encountered during parsing.
/// * `elapsed_time` - The total time elapsed during parsing in seconds.
///
/// # Example
/// ```rust
/// let errors = 2;
/// let warnings = 5;
/// let elapsed = 0.42;
/// display_parsing_result(errors, warnings, elapsed);
/// // Output: "Finished 2 error(s), 5 warning(s) in 0.42s"
/// ```
fn display_parsing_result(error_count: usize, warning_count: usize, elapsed_time: f32) {
    println!(
        "{} {} error(s), {} warning(s) in {:.2}s",
        "Finished".green().bold(),
        error_count,
        warning_count,
        elapsed_time
    );
}

/// Validates the arguments for the `parse` subcommand.
///
/// This function enforces the following rules for the CLI:
/// 1. If multiple input files are provided, `-o/--output` is forbidden.
/// 2. The output directory argument `-d/--out-dir` is optional for multiple files
///    because a default (`"."`) is provided.
/// 3. The format argument `-f/--format` is always optional and defaults to JSON.
///
/// # Arguments
/// * `matches` - The parsed command-line arguments from `clap`.
///
/// # Returns
/// * `Ok(())` if all validations pass.
/// * `Err(clap::Error)` if any validation rule is violated.
///
/// # Example
/// ```rust
/// use aiplan4rust::cli::parse::{build_parse_subcommand, validate_parse_args};
/// let matches = build_parse_subcommand().get_matches();
/// validate_parse_args(&matches)?;
/// ```
fn validate_parse_args(matches: &ArgMatches) -> Result<(), clap::Error> {
    // Count the number of input files provided
    let file_count = matches
        .get_many::<String>(FILES_ARG)
        .map(|v| v.len())
        .unwrap_or(0);

    // Check if the output file argument was provided
    let output_provided = matches.contains_id(OUTPUT_ARG);

    // If multiple files are provided, -o is forbidden
    if file_count > 1 && output_provided {
        return Err(clap::Error::raw(
            ErrorKind::ArgumentConflict,
            "`-o/--output` cannot be used with multiple input files; use `-d/--out-dir` instead",
        ));
    }

    // No need to require -d/--out-dir because we have a default value
    Ok(())
}
