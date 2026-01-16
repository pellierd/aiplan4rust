//! This module provides the CLI grounding workflow for `aiplan4rust`.
//!
//! It defines functions to handle the `ground` subcommand, validate grounded inputs,
//! perform grounding of a domain + problem pair, and manage output serialization,
//! diagnostics reporting, and timing statistics.

use crate::aiplan4rust::cli::cli::{CURRENT_DIR, FILES_ARG, FORMAT_ARG, OUTPUT_ARG, OUT_DIR_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::artefact::source::Source;
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{Frontend, Renderer, Severity};
use clap::ArgMatches;
use std::fs;
use std::path::PathBuf;
use colored::Colorize;
use crate::aiplan4rust::grounding::problem::Problem;

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

    let domain_file = PathBuf::from(&files[0]);
    let problem_file = PathBuf::from(&files[1]);

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

    // --- Validate inputs ---
    //let domain = validate_domain(&domain_file)?;
    //let problem = validate_problem(&problem_file)?;

    // --- Execute grounding workflow ---
    //ground_inputs(&domain, &problem, format, &out_dir, &output_opt)?;

    Ok(())
}

/*/// Ground a domain + problem pair and serialize output.
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
    if domain.is_parsed_domain() {
        ground_parsed(domain, problem, format, &output_path)?;
    } else {
        ground_raw(domain, problem, format, &output_path)?;
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    println!(
        "{} Grounding completed in {:.2}s",
        "Finished".green().bold(),
        elapsed
    );

    Ok(())
}

/// Ground parsed (IR) domain/problem input.
fn ground_parsed(
    domain: &Source,
    problem: &Source,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    let frontend = Frontend::new();
    let mut result = frontend.ground_parsed_input(domain, problem)?;

    // Display diagnostics
    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display()?;

    // Serialize output if successful
    if let Some(grounded) = result.take_grounded_problem() {
        save_ground_output(grounded, format, output)?;
    }

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
    let mut result = frontend.ground_raw_input(domain, problem)?;

    let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
    renderer.display()?;

    if let Some(grounded) = result.take_grounded_problem() {
        save_ground_output(grounded, format, output)?;
    }

    Ok(())
}

/// Save grounded problem to disk.
fn save_ground_output<P: Into<PathBuf>>(
    grounded_problem: Problem,
    format: SerdeFormat,
    output_path: P,
) -> Result<(), CliError> {
    let output_path = output_path.into();

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize and write to disk
    let content = Problem(grounded_problem, format);
    let output = crate::aiplan4rust::artefact::Artefact::new_ir(output_path.clone(), content);
    output.write()?;

    println!(
        "{} Grounded output saved to {}",
        "===>".blue().bold(),
        output_path.display()
    );

    Ok(())
}

/// Validate domain file for grounding.
fn validate_domain(path: &PathBuf) -> Result<Source, CliError> {
    match Source::try_from_path(path) {
        Ok(s) if s.is_parsed_domain() || s.is_raw() => Ok(s),
        Ok(_) => Err(CliError::custom("Domain file must be raw or parsed domain")),
        Err(e) => Err(CliError::custom(format!("Cannot read domain file: {}", e))),
    }
}

/// Validate problem file for grounding.
fn validate_problem(path: &PathBuf) -> Result<Source, CliError> {
    match Source::try_from_path(path) {
        Ok(s) if s.is_parsed_problem() || s.is_raw() => Ok(s),
        Ok(_) => Err(CliError::custom("Problem file must be raw or parsed problem")),
        Err(e) => Err(CliError::custom(format!("Cannot read problem file: {}", e))),
    }
}

/// Compute default grounded output path (placeholder implementation)
fn default_grounded_output_path(domain: &PathBuf, problem: &PathBuf, out_dir: &PathBuf) -> Result<PathBuf, CliError> {
    let filename = problem
        .file_stem()
        .ok_or_else(|| CliError::custom("Problem file has no stem"))?;
    Ok(out_dir.join(format!("{}.grounded.json", filename.to_string_lossy())))
}*/
