use clap::ArgMatches;
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeSerializable};
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::cli::handle::{ensure_parent_dir_exists, generate_default_output_filename};
use crate::aiplan4rust::serialization::header::Header;
use crate::aiplan4rust::source::Source;

/// Handles the `link` command logic.
///
/// This function retrieves the domain and problem files from the CLI arguments,
/// validates that exactly two files are provided, and calls the `link` function
/// to combine them into the specified output format.
///
/// # Arguments:
/// - `matches`: Parsed command-line arguments for the `link` subcommand.
/// Handles the `link` subcommand for the CLI.
///
/// This function validates input files, determines whether they are already parsed,
/// generates the output filename if not provided, and then either links directly
/// or parses then links depending on the file state.
pub fn handle_link_command(matches: &ArgMatches) -> Result<(), CliError> {
    // Collect input files from the CLI arguments
    let files: Vec<String> = match matches.get_many::<String>(FILES_ARG) {
        Some(values) => values.cloned().collect(), // Clone each value into a Vec<String>
        None => return Err(CliError::missing_input_files()), // Error if no files provided
    };

    // Ensure exactly two files are provided (domain + problem)
    if files.len() != 2 {
        return Err(CliError::InvalidFileCount);
    }

    // Assign domain and problem file paths
    let domain_file = &files[0];  // First file is domain
    let problem_file = &files[1]; // Second file is problem

    // Get the output serialization format (default: JSON)
    let format = *matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();

    // Determine the output filename
    let output = match matches.get_one::<String>(OUTPUT_ARG) {
        Some(s) => s.clone(), // Use user-provided output filename
        None => generate_default_output_filename(domain_file, Some(problem_file), format, None)?, // Auto-generate if not provided
    };

    // Ensure that the parent directory of the output file exists
    ensure_parent_dir_exists(&output);

    // Detect whether each input file is already serialized (parsed)
    let domain_parsed = Header::is_serialized_file(domain_file);
    let problem_parsed = Header::is_serialized_file(problem_file);

    let domain = Source::from_path_str(domain_file)?;
    let problem = Source::from_path_str(problem_file)?;
    // Decide action based on file parsing state
    match (domain_parsed, problem_parsed) {
        (true, true) => {
            // Both files are parsed → link only
            link(&domain, &problem, format, &output)?
        }
        (false, false) => {
            // Both files are raw → parse and then link
            link_from_files(&domain, &problem, format, &output)?
        }
        _ => {
            // Mixed state: one parsed, one raw → inconsistent
            return Err(CliError::inconsistent_files());
        }
    }

    Ok(())
}

fn link(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &str,
) -> Result<(), CliError> {
    // Create a frontend instance
    let frontend = Frontend::new();

    // Perform linking, propagate any errors
    let linker_result = frontend.link(domain, problem)?;

    // If linking produced a semantic context, serialize it
    if let Some(planning_task) = linker_result.linked_semantic_context() {
        planning_task.serialize_to_file(format, output)?; // propagate SerializationError as CliError
        println!("Output saved to {}", output);
    } else {
        // Otherwise, render diagnostics
        let mut renderer = Renderer::new(
            linker_result.diagnostic_manager(),
            linker_result.interner(),
        );
        renderer.display()?;
    }

    Ok(())
}

pub fn link_from_files(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &str,
) -> Result<(), CliError> {
    use std::time::Instant;

    let start_time = Instant::now();

    println!(
        "{:>10} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain.path_str(),
        problem.path_str()
    );

    let frontend = Frontend::new();

    // Parse domain and problem files
    let result = frontend.parse(domain, problem)?; // AiplanError se convertit en CliError

    // Display diagnostics (propagation via DiagnosticError)
    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display()?; // DiagnosticError se convertit en CliError

    // Count errors and warnings
    let dm = result.diagnostic_manager();
    let error_count = dm.count_diagnostics_of_severity(Severity::Error);
    let warning_count = dm.count_diagnostics_of_severity(Severity::Warning);

    // Elapsed time
    let elapsed = start_time.elapsed().as_secs_f32();

    println!(
        "{} {} error(s), {} warning(s) in {:.2}s",
        "Finished".green().bold(),
        error_count,
        warning_count,
        elapsed
    );

    if error_count > 0 {
        println!(
            "{} No output file produced due to errors.",
            "===>".blue().bold()
        );
    } else if let Some(lifted_problem) = result.lifted_problem() {
        // Serialize the lifted problem
        lifted_problem.serialize_to_file(format, output)?; // SerializationError se convertit en CliError
        println!(
            "{} Output saved to {}",
            "===>".blue().bold(),
            output
        );
    }

    Ok(())
}
