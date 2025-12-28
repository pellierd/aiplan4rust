use std::fs;
use clap::error::ErrorKind;
use clap::ArgMatches;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::{AnalyzerResult, Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::artefact::error::ArtefactError;
use crate::aiplan4rust::artefact::{IRContent, Artefact};
use crate::aiplan4rust::artefact::source::Source;
use crate::aiplan4rust::cli::check::check_parse_args;
use crate::aiplan4rust::cli::path::default_parsed_output_path;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::serialization::SerdeFormat;
use crate::aiplan4rust::syntax::ast::AstKind;

/// Handles the `parse` CLI subcommand.
///
/// This function coordinates the entire parsing workflow for one or more input files,
/// delegating the heavy lifting to `process_and_parse_inputs`.
///
/// # Step-by-step behavior
///
/// 1. **Validate CLI arguments** using `check_parse_args`.
/// 2. **Collect input files** specified by the user and convert them into `PathBuf`.
/// 3. **Retrieve the output format**; defaults to `JSON` if not explicitly provided.
/// 4. **Determine the output directory** for multiple files, defaulting to the current directory if unspecified.
/// 5. **Delegate the parsing and statistics accumulation** to `process_and_parse_inputs`.
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
    // Validate CLI arguments
    check_parse_args(matches)?;

    // Collect input files
    let files: Vec<PathBuf> = matches
        .get_many::<String>(FILES_ARG)
        .ok_or_else(|| {
            clap::Error::raw(
                ErrorKind::MissingRequiredArgument,
                "No input files provided"
            )
        })?
        .map(PathBuf::from)
        .collect();

    // Retrieve the output format
    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .ok_or_else(|| {
            clap::Error::raw(
                ErrorKind::MissingRequiredArgument,
                "No output format provided"
            )
        })?;

    // Determine the output directory
    let out_dir = matches
        .get_one::<String>(OUT_DIR_ARG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CURRENT_DIR));

    // Optional output path
    let output_opt = matches.get_one::<String>(OUTPUT_ARG).map(PathBuf::from);

    // Validate raw source files
    let source_files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let sources = filter_raw_sources(&source_files)?;

    if sources.is_empty() {
        println!("Warning: no valid sources files found, nothing to do");
        return Ok(());
    }

    // Delegate processing to the factorized function
    parse_inputs(&sources, format, &out_dir, &output_opt)?;

    Ok(())
}

/// Parses a list of raw PDDL/HDDL input sources and writes the parsed outputs.
///
/// This function processes each `Source` in `sources`:
/// 1. Determines the output path: uses `output_opt` if provided, otherwise
///    generates a default parsed output path in `out_dir`.
/// 2. Parses the raw input using `parse_from_raw_input`.
/// 3. Accumulates diagnostics statistics (warnings and errors).
///
/// After all sources are processed, a summary is printed if multiple files
/// were parsed.
///
/// # Arguments
/// * `sources` - A slice of validated raw input sources to parse.
/// * `format` - The output serialization format.
/// * `out_dir` - The directory where parsed output files should be written.
/// * `output_opt` - Optional single output path (used if only one input source).
///
/// # Errors
/// Returns a `CliError` if parsing any of the sources fails.
///
/// # Example
/// ```rust
/// # use std::path::Path;
/// # use crate::aiplan4rust::cli::handle::parse_inputs;
/// # use crate::aiplan4rust::source::Source;
/// # use crate::aiplan4rust::serialization::serde::SerdeFormat;
/// # use crate::aiplan4rust::cli::CliError;
/// let sources: Vec<Source> = vec![/* ... */];
/// let out_dir = Path::new("out/");
/// let output_opt = None;
/// let format = SerdeFormat::Json;
/// parse_inputs(&sources, out_dir, &output_opt, format)?;
/// ```
pub fn parse_inputs(
    sources: &Vec<Source>,
    format: SerdeFormat,
    out_dir: &Path,
    output_opt: &Option<PathBuf>,
) -> Result<(), CliError> {
    use std::time::Instant;

    let mut total_warnings = 0usize;
    let mut total_errors = 0usize;
    let start_time = Instant::now();

    for source in sources {
        // Determine output path
        let output_path = match output_opt {
            Some(path) => path.to_path_buf(),
            None => default_parsed_output_path(source.path(), out_dir)?,
        };

        // Parse the raw input
        let result = parse_from_raw_input(&source, output_path, format)?;

        // Accumulate diagnostics
        total_warnings += result
            .diagnostic_manager()
            .count_diagnostics_of_severity(Severity::Warning);
        total_errors += result
            .diagnostic_manager()
            .count_diagnostics_of_severity(Severity::Error);
    }

    // Print summary if multiple files were processed
    if sources.len() > 1 {
        let total_time = start_time.elapsed().as_secs_f32();
        println!(
            "\nFinished {} error(s), {} warning(s) in {:.2}s",
            total_errors,
            total_warnings,
            total_time
        );
    }

    Ok(())
}


/// Filters a list of file paths, returning only those that are valid raw PDDL/HDDL sources.
///
/// This function iterates over the provided file paths, attempting to read each one as a
/// `Source`. Only sources that are classified as "raw" are kept. Non-file paths, unreadable
/// files, or sources that are not raw are skipped, and a warning message is printed for each.
///
/// # Arguments
///
/// * `source_paths` - A vector of `PathBuf` representing the candidate input files.
///
/// # Returns
///
/// * `Ok(Vec<Source>)` - A vector containing all valid raw sources.
/// * `Err(CliError)` - If an underlying I/O or parsing error occurs while reading a file.
///
/// # Behavior
///
/// 1. Checks that the path exists and is a regular file; otherwise prints a warning and skips it.
/// 2. Attempts to convert the file to a `Source`; if reading fails, prints a warning and skips it.
/// 3. Checks if the source is raw; if not, prints a warning describing the source type.
///
/// # Warnings
///
/// Each skipped file triggers a `println!` message indicating why it was ignored. Possible
/// messages include:
/// - Not a file
/// - Failed to read file
/// - Not a raw PDDL/HDDL input (with the file kind)
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
/// use crate::aiplan4rust::cli::handle::filter_raw_sources;
///
/// let files = vec![PathBuf::from("domain.pddl"), PathBuf::from("problem.lift")];
/// let valid_sources = filter_raw_sources(&files).unwrap();
/// assert!(valid_sources.len() >= 0); // Only raw sources are included
/// ```
fn filter_raw_sources(source_paths: &Vec<PathBuf>) -> Result<Vec<Source>, CliError> {
    let mut valid_inputs = Vec::new();

    for path in source_paths {
        // 1. Check if the path is a regular file
        if !path.is_file() {
            println!("Warning: '{}' is not a file — ignored", path.display());
            continue;
        }

        // 2. Read the file into a Source
        let source = match Source::try_from_path(path) {
            Ok(src) => src,
            Err(e) => {
                println!("Warning: failed to read '{}': {} — ignored", path.display(), e);
                continue;
            }
        };

        // 3. Check if the input is raw
        if source.is_raw() {
            valid_inputs.push(source);
        } else {
            // Determine kind for message
            let kind = match () {
                _ if source.is_ir() => "IR file",
                _ if source.is_text() => "text file",
                _ if source.is_binary() => "binary file",
                _ => "unknown content",
            };

            println!(
                "Warning: '{}' is not a valid PDDL/HDDL input ({}) — ignored",
                path.display(),
                kind
            );
        }
    }

    Ok(valid_inputs)
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
fn parse_from_raw_input(
    input: &Source,
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
    let mut result = frontend.parse_from_raw_input(input)?;

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
) -> Result<(), ArtefactError> {
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
    let output = Artefact::new_ir(output_path.clone(), ir_content);

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
