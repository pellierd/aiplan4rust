//! This module provides the CLI grounding workflow for `aiplan4rust`.
//!
//! It defines functions to handle the `ground` subcommand, validate grounded inputs,
//! perform grounding of a domain + problem pair, and manage output serialization,
//! diagnostics reporting, and timing statistics.

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::cli::io::artefact::error::ArtefactError;
use crate::aiplan4rust::cli::io::artefact::path::default_grounded_output_path;
use crate::aiplan4rust::cli::io::artefact::source::Source;
use crate::aiplan4rust::cli::io::artefact::{Artefact, IRContent};
use crate::aiplan4rust::cli::io::serialization::serde::SerdeFormat;
use crate::aiplan4rust::compiler::grounding::Problem;
use crate::{Frontend, Renderer};
use clap::ArgMatches;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// Handles the `ground` CLI subcommand.
///
/// This function orchestrates the workflow for grounding a domain + problem input:
/// 1. Validate CLI arguments
/// 2. Read input files
/// 3. Validate and filter inputs
/// 4. Execute grounding workflow
/// 5. Serialize outputs and print diagnostics
pub fn handle_ground_command(matches: &ArgMatches) -> Result<(), CliError> {
    // --- Collect input files ---
    let files: Vec<String> = matches
        .get_many::<String>(FILES_ARG)
        .ok_or_else(CliError::missing_input_files)?
        .cloned()
        .collect();

    // --- Determine output format ---
    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .unwrap_or(&SerdeFormat::Json);

    // --- Optional output path ---
    let output_opt = matches.get_one::<String>(OUTPUT_ARG).map(PathBuf::from);

    // --- Determine output directory ---
    let out_dir = matches
        .get_one::<String>(OUT_DIR_ARG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CURRENT_DIR));

    let domain_file = PathBuf::from(&files[0]);
    let domain = match filter_domain(&domain_file) {
        Some(d) => d,
        None => {
            println!("Warning: no valid domain found, ground command skipped");
            return Ok(());
        }
    };

    let problem_file = PathBuf::from(&files[1]);
    let problem = match filter_problem(&problem_file) {
        Some(problem) => problem,
        None => {
            println!("Warning: no valid problem found, ground command skipped");
            return Ok(());
        }
    };

    // --- Execute grounding workflow ---
    ground_inputs(&domain, &problem, format, &out_dir, &output_opt)?;

    Ok(())
}

/// Ground a domain + problem pair and serialize output.
fn ground_inputs(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    out_dir: &PathBuf,
    output_opt: &Option<PathBuf>,
) -> Result<(), CliError> {
    use std::time::Instant;
    let start_time = Instant::now();

    let output_path: PathBuf = match output_opt {
        Some(path) => path.clone(),
        None => default_grounded_output_path(domain.path(), problem.path(), out_dir)?,
    };

    // Perform grounding
    ground_raw(domain, problem, format, &output_path)?;

    let elapsed = start_time.elapsed().as_secs_f32();
    println!(
        "{} Grounding completed in {:.2}s",
        "Finished".green().bold(),
        elapsed
    );

    Ok(())
}

/// Ground raw domain/problem input.
fn ground_raw(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    let frontend = Frontend::new();
    let mut result = frontend.ground_from_raw_input(domain, problem)?;

    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display()?;

    if let Some(problem) = result.take_problem() {
        save_ground_output(problem, format, output)?;
    }

    Ok(())
}

/// Save grounded problem to disk.
pub fn save_ground_output<P: Into<PathBuf>>(
    problem: Problem,
    format: SerdeFormat,
    output_path: P,
) -> Result<(), ArtefactError> {
    let output_path = output_path.into();

    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Wrap the grounded problem in IRContent (or a new variant GroundedProblem if defined)
    let content = IRContent::GroundedProblem(problem, format); // <-- adapter le enum IRContent
    let output = Artefact::new_ir(output_path.clone(), content);

    // Write to disk
    output.write()?;

    // Display absolute path
    let absolute_output = output_path
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| output_path.display().to_string());

    println!(
        "{} Output file produced ({})",
        "===>".blue().bold(),
        absolute_output
    );

    Ok(())
}

/// Lit et valide le domain : doit être RawDomain ou ParsedDomain
/// Validates a domain file.
///
/// This function attempts to read the domain file from the given path and checks
/// if it is either a RawDomain or ParsedDomain. If the domain cannot be read or
/// is of an incompatible typing, a warning is printed and `None` is returned.
///
/// # Parameters
///
/// * `path` - The path to the domain file to validate.
///
/// # Returns
///
/// * `Some(Input)` - If the domain is successfully read and is of a valid typing.
/// * `None` - If the domain could not be read or is not a valid Raw/Parsed domain.
pub fn filter_domain(path: &PathBuf) -> Option<Source> {
    match Source::try_from_path(path) {
        // Domain successfully read and has a valid typing
        Ok(d) if d.is_domain() && d.is_raw() => Some(d),

        // Domain read but typing is invalid
        Ok(d) => {
            println!(
                "Warning: domain '{}' ignored: must be a RawDomain",
                d.path().display()
            );
            None
        }

        // Domain file could not be read
        Err(e) => {
            println!(
                "Warning: unable to read domain file '{}': {}",
                path.display(),
                e
            );
            None
        }
    }
}

/// * `None` - If the domain could not be read or is not a valid Raw/Parsed domain.
pub fn filter_problem(path: &PathBuf) -> Option<Source> {
    match Source::try_from_path(path) {
        // Domain successfully read and has a valid typing
        Ok(d) if d.is_problem() && d.is_raw() => Some(d),

        // Domain read but typing is invalid
        Ok(d) => {
            println!(
                "Warning: problem '{}' ignored: must be a raw problem",
                d.path().display()
            );
            None
        }

        // Domain file could not be read
        Err(e) => {
            println!(
                "Warning: unable to read problem file '{}': {}",
                path.display(),
                e
            );
            None
        }
    }
}
