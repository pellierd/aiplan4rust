use std::fs;
use clap::error::ErrorKind;
use clap::ArgMatches;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{AnalyzerResult, Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::io::{Extension, IRContent, Output};
use crate::aiplan4rust::io::error::IOError;
use crate::aiplan4rust::io::input::Input;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::syntax::ast::AstKind;

/// Handles the logic for the `parse` CLI subcommand.
///
/// # Step-by-step behavior
///
/// 1. **Validate CLI arguments** using `validate_parse_args`.
/// 2. **Collect input files** specified by the user and convert them to `PathBuf`.
/// 3. **Retrieve the output format**; defaults to `JSON` if not explicitly provided.
/// 4. **Determine the output directory** for multiple files, defaulting to the current directory if unspecified.
/// 5. **Loop through each input file**:
///     - If a single input file and `-o/--output` is provided, use it as the output path.
///     - Otherwise, generate a default output path based on the input file, optional problem file, extension, and output directory.
/// 6. **Parse each input file** and save the output using `parse_from_files`.
///    - Directory creation and file writing are handled internally by `parse_from_files` and `save_parse_output`.
///
/// # Behavior details
///
/// - **Single input file**:
///     - Uses `-o/--output` if provided.
///     - Otherwise, generates a default filename in the current directory or specified output directory.
/// - **Multiple input files**:
///     - `-o/--output` should not be provided (validation ensures this).
///     - Requires `-d/--out-dir` to specify an output directory, defaults to the current directory.
///     - Generates output filenames for each input file inside the output directory.
///
/// # Arguments
///
/// * `matches` - A reference to `ArgMatches` containing parsed CLI arguments for the `parse` subcommand.
///
/// # Returns
///
/// Returns `Ok(())` if all input files are parsed and outputs are written successfully.
/// Returns `Err(CliError)` if:
/// - No input files are provided,
/// - The output format is missing,
/// - Parsing or saving any output fails.
///
/// # Example
///
/// ```rust
/// let matches = build_parse_subcommand().get_matches();
/// handle_parse_command(&matches)?;
/// ```
pub fn handle_parse_command(matches: &ArgMatches) -> Result<(), CliError> {
    use std::time::Instant;

    check_parse_args(matches)?;

    let input_paths: Vec<PathBuf> = matches
        .get_many::<String>(FILES_ARG)
        .ok_or_else(|| CliError::invalid_argument("No input files provided"))?
        .map(PathBuf::from)
        .collect();

    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .ok_or_else(|| CliError::invalid_argument("No output format provided"))?;

    let out_dir = matches
        .get_one::<String>(OUT_DIR_ARG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CURRENT_DIR));

    // Statistiques globales
    let mut files_parsed = 0usize;
    let mut files_ignored = 0usize;
    let mut total_warnings = 0usize;
    let mut total_errors = 0usize;
    let start_time = Instant::now();

    // Première passe : afficher les fichiers ignorés
    let mut inputs_to_parse: Vec<(Input, PathBuf)> = Vec::new();

    for input_path in &input_paths {
        let output_path = match matches.get_one::<String>(OUTPUT_ARG) {
            Some(s) if input_paths.len() == 1 => PathBuf::from(s),
            _ => Output::default_output_path(input_path, None, Extension::Parsed, Some(&out_dir))?,
        };

        match read_input_with_warning(input_path)? {
            Some(input) => inputs_to_parse.push((input, output_path)),
            None => files_ignored += 1,
        }
    }

    // Deuxième passe : parse et accumulateur de stats
    for (input, output_path) in inputs_to_parse {
        let parse_start = Instant::now();
        let result = parse(input, output_path.clone(), format)?;

        // Accumuler stats
        total_warnings += result
            .diagnostic_manager()
            .count_diagnostics_of_severity(Severity::Warning);
        total_errors += result
            .diagnostic_manager()
            .count_diagnostics_of_severity(Severity::Error);

        files_parsed += 1;
    }

    // Résumé global
    if input_paths.len() > 1 {
        let total_time = start_time.elapsed().as_secs_f32();
        println!(
            "\n{} {} file(s) parsed successfully, {} file(s) ignored, {} error(s), {} warning(s) in {:.2}s",
            "Finished".green().bold(),
            files_parsed,
            files_ignored,
            total_errors,
            total_warnings,
            total_time
        );
    }

    Ok(())
}


/// Tries to read a PDDL/HDDL input file.
/// Returns `Some(Input)` if successful, `None` if the input is not raw (warning).
/// Returns a fatal `CliError` for other errors.
fn read_input_with_warning(input_path: &Path) -> Result<Option<Input>, CliError> {
    // Vérifier d'abord si c'est un fichier régulier
    if !input_path.is_file() {
        println!(
            "{} '{}' is not a file — ignored",
            "warning:".yellow().bold(),
            input_path.display()
        );
        return Ok(None);
    }

    // Lire le fichier et détecter son type
    let input = Input::read_from_file(input_path)?;

    // Vérifier si c'est du Raw
    if input.is_raw() {
        Ok(Some(input))
    } else {
        // Non-fatal, print a warning et skip
        let kind = if input.is_ir() {
            "IR file"
        } else if input.is_unknown_text() {
            "unknown text file"
        } else if input.is_binary_unknown() {
            "binary file"
        } else {
            "unknown content"
        };

        println!(
            "{} '{}' is not a valid PDDL/HDDL input ({}) — ignored",
            "warning:".yellow().bold(),
            input_path.display(),
            kind
        );

        Ok(None)
    }
}

/// Parses a single PDDL/HDDL input file and optionally writes its semantic context to an output file.
///
/// This function orchestrates the full parsing workflow:
/// 1. Resolves and prints a parsing start message including the absolute path of the input file.
/// 2. Reads the input file into memory using `Input::read_from_file`.
/// 3. Parses the file using the `Frontend` parser.
/// 4. Renders diagnostics (errors and warnings) to the console.
/// 5. Displays a summary including the number of errors, warnings, and elapsed time.
///    - If errors exist, a message is printed and no output file is produced.
/// 6. If parsing succeeds with no blocking errors and a semantic context is available,
///    the context is serialized to the specified output file in the chosen format.
///
/// # Arguments
///
/// * `input_path` - Path to the PDDL/HDDL input file to parse. Accepts any type implementing `AsRef<Path>`.
/// * `output_path` - Path to the output file where the serialized semantic context will be written.
/// * `format` - The serialization format for the output file (e.g., JSON, YAML, TOML).
///
/// # Returns
///
/// * `Ok(())` if parsing and optional serialization succeed.
/// * `Err(CliError)` if reading the file, parsing, rendering diagnostics, or writing the output fails.
///
/// # Behavior
///
/// - Parsing errors are displayed to the console, and if any errors exist, no output file is written.
/// - Semantic context is only serialized if parsing succeeds with no blocking errors.
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
/// use aiplan4rust::cli::parse::*;
/// use aiplan4rust::serialization::serde::SerdeFormat;
///
/// let input_file = "domain.pddl";
/// let output_file = PathBuf::from("domain.json");
/// parse_from_files(input_file, &output_file, SerdeFormat::Json).unwrap();
/// // Console output:
/// // "Finished 0 error(s), 1 warning(s) in 0.42s"
/// ```
fn parse(
    input: Input,
    output_path: PathBuf,
    format: SerdeFormat,
) -> Result<AnalyzerResult, CliError> {
    // Start a timer to measure parsing duration
    let start_time = Instant::now();

    // Display a message indicating parsing has started
    display_parsing_start(input.path());

    // Initialize the frontend parser
    let frontend = Frontend::new();

    // Parse the already validated input
    let mut result = frontend.parse_file(&input)?;

    // Compute elapsed parsing time
    let elapsed_time = start_time.elapsed().as_secs_f32();

    // Render diagnostics and display parsing summary
    display_parsing_result(&result, elapsed_time)?;

    // Serialize semantic context if available
    if let Some(context) = result.take_semantic_context() {
        save_parse_output(context, format, output_path)?;
    }

    Ok(result)
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
/// * `input_path` - Path to the input PDDL/HDDL file to parse.
///
/// # Returns
/// * `String` - The absolute path of the input file, or the original path if canonicalization fails.
///
/// # Example
/// ```rust
/// use std::path::PathBuf;
/// let input_file = PathBuf::from("domain.pddl");
/// let absolute_path = display_parsing_start(&input_file);
/// println!("Parsing started for: {}", absolute_path);
/// ```
fn display_parsing_start<P: AsRef<Path>>(input_path: P) -> String {
    let input_path_ref = input_path.as_ref();

    // Resolve the absolute path; fallback to the original path if canonicalization fails
    let path = input_path_ref
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| input_path_ref.display().to_string());

    // Use the version from Cargo.toml automatically
    let version = env!("CARGO_PKG_VERSION");

    // Print the parsing start message
    println!(
        "\n{} aiplan4rust v{} ({})",
        "Parsing".green().bold(),
        version,
        path
    );

    path
}

/// Displays a summary of the parsing results, including the number of errors,
/// warnings, and the total elapsed time for parsing.
///
/// This function performs the following actions:
/// 1. Renders and displays diagnostics using the `Renderer`.
/// 2. Counts the number of errors and warnings from the `AnalyzerResult`.
/// 3. Prints a formatted summary message to the console in the following format:
///    `"Finished <error_count> error(s), <warning_count> warning(s) in <elapsed_time>s"`.
///
/// # Arguments
/// * `result` - A reference to the `AnalyzerResult` containing diagnostics and semantic context.
/// * `elapsed_time` - The total time elapsed during parsing in seconds.
///
/// # Returns
/// Returns `Ok(())` if the diagnostics were displayed successfully, or a `CliError`
/// if an error occurs during rendering.
///
/// # Example
/// ```rust
/// let result: AnalyzerResult = parse_file("input_file").unwrap();
/// let elapsed = 0.42;
/// display_parsing_result(&result, elapsed).unwrap();
/// // Output: "Finished 2 error(s), 5 warning(s) in 0.42s"
/// ```
fn display_parsing_result(
    result: &AnalyzerResult,
    elapsed_time: f32,
) -> Result<(), CliError> {
    // Render diagnostics
    let mut renderer = Renderer::new(
        result.diagnostic_manager(),
        result.interner(),
    );
    renderer.display()?;

    // Count errors and warnings
    let diagnostic_manager = result.diagnostic_manager();
    let error_count =
        diagnostic_manager.count_diagnostics_of_severity(Severity::Error);
    let warning_count =
        diagnostic_manager.count_diagnostics_of_severity(Severity::Warning);

    println!(
        "{} {} error(s), {} warning(s) in {:.2}s",
        "Finished".green().bold(),
        error_count,
        warning_count,
        elapsed_time
    );

    // Display "no output" message if there are errors
    if error_count > 0 {
        println!(
            "{} No output file produced due to errors.",
            "===> ".blue().bold()
        );
    }

    Ok(())
}

/// Checks the arguments for the `parse` subcommand.
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
fn check_parse_args(matches: &ArgMatches) -> Result<(), clap::Error> {
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

    Ok(())
}

/// Saves a semantic context to a file by creating an `Output::IR` and writing it.
///
/// This function ensures that the parent directories exist, serializes the content,
/// and prints the absolute path of the output file.
///
/// # Arguments
///
/// * `context` - The semantic context to save (e.g., a parsed PDDL/HDDL domain or problem).
/// * `format` - The serialization format to use (JSON, YAML, TOML, etc.).
/// * `output_path` - The path where the output file will be written.
///
/// # Errors
///
/// Returns an `IOError` if directory creation or file writing fails.
///
/// # Example
///
/// ```rust
/// save_parse_output(context, SerdeFormat::Json, PathBuf::from("out.json"))?;
/// ```
pub fn save_parse_output<P: Into<PathBuf>>(
    context: SemanticContext,
    format: SerdeFormat,
    output_path: P,
) -> Result<(), IOError> {
    // Convert the generic path type into a PathBuf
    let output_path = output_path.into();

    // Create all parent directories if they do not exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?; // creates all parent directories
    }

    // Dynamically determine the IR kind based on the semantic context
    // Could be improved with a `context.kind()` method
    let kind = context.syntax_tree().try_root().unwrap().kind();  // either ParsedDomain or ParsedProblem

    // Build the IR content enum variant based on the kind
    let ir_content = match kind {
        AstKind::Domain => IRContent::ParsedDomain(context, format),
        AstKind::Problem => IRContent::ParsedProblem(context, format),
        _ => unreachable!("SemanticContext cannot be a LiftedProblem"),
    };

    // Create an Output::IR object for writing
    let output = Output::new_ir(output_path.clone(), ir_content);

    // Write the content to disk
    output.write()?;

    // Resolve the absolute path for display purposes
    let absolute_output = output_path
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| output_path.display().to_string());

    // Print a confirmation message with the absolute path
    println!(
        "{} Output file produced ({})",
        "===> ".blue().bold(),
        absolute_output
    );

    Ok(())
}
