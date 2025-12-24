use std::fs;
use std::path::PathBuf;
use clap::ArgMatches;
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::cli::handle::ensure_parent_dir_exists;
use crate::aiplan4rust::io::{Extension, IRContent, Output};
use crate::aiplan4rust::io::error::IOError;
use crate::aiplan4rust::io::input::Input;
use crate::aiplan4rust::lir::problem::LiftedProblem;

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
    // Collecte les fichiers depuis les arguments CLI
    let files: Vec<String> = match matches.get_many::<String>(FILES_ARG) {
        Some(values) => values.cloned().collect(),
        None => return Err(CliError::missing_input_files()),
    };

    // Récupère le format de sortie (default: JSON)
    let format = *matches
        .get_one::<SerdeFormat>(FORMAT_ARG)
        .ok_or_else(|| CliError::invalid_argument("No output format provided"))?;

    let domain_file = PathBuf::from(&files[0]);
    let problem_file = PathBuf::from(&files[1]);

    // Détermine le nom du fichier de sortie
    let output_path = match matches.get_one::<String>(OUTPUT_ARG) {
        Some(s) => PathBuf::from(s),
        None => Output::default_output_path(
            &domain_file,
            Some(&problem_file),
            Extension::Lifted,
            None,
        )?,
    };

    ensure_parent_dir_exists(&output_path);

    // Charge les sources
    let domain = Input::read_from_file(domain_file)?;
    let problem = Input::read_from_file(problem_file)?;

    // ---------------------------------------------------------------------
    // Deux fichiers IR → link direct
    // ---------------------------------------------------------------------
    if domain.is_ir() && problem.is_ir() {
        link_from_parsed_input(&domain, &problem, format, &output_path)?;
    }
    // ---------------------------------------------------------------------
    // Deux fichiers Raw → parse + link
    // ---------------------------------------------------------------------
    else if domain.is_raw() && problem.is_raw() {
        link_from_raw_input(&domain, &problem, format, &output_path)?;
    }
    // ---------------------------------------------------------------------
    // Cas non supportés → erreur
    // ---------------------------------------------------------------------
    else {
        return Err(CliError::inconsistent_files());
    }

    Ok(())
}

fn link_from_parsed_input(
    domain: &Input,
    problem: &Input,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    // Create a frontend instance
    let frontend = Frontend::new();

    // Perform linking, propagate any errors
    let mut builder_result = frontend.link_from_parsed_input(domain, problem)?;

    // If linking produced a semantic context, serialize it
    if let Some(lifted_problem) = builder_result.take_lifted_problem() {
        save_link_output(lifted_problem, format, output)?;
        println!("Output saved to {}", output.to_string_lossy());
    } else {
        // Otherwise, render diagnostics
        let mut renderer = Renderer::new(
            builder_result.diagnostic_manager(),
            builder_result.interner(),
        );
        renderer.display()?;
    }

    Ok(())
}

pub fn link_from_raw_input(
    domain: &Input,
    problem: &Input,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    use std::time::Instant;

    let start_time = Instant::now();

    println!(
        "{} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain.path().to_string_lossy(),
        problem.path().to_string_lossy()
    );

    let frontend = Frontend::new();

    // Parse domain and problem files
    let mut result = frontend.link_from_raw_input(domain, problem)?; // AiplanError se convertit en CliError

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
    } else if let Some(lifted_problem) = result.take_lifted_problem() {
        // Serialize the lifted problem
        save_link_output(lifted_problem, format, output)?;
        println!(
            "{} Output saved to {}",
            "===>".blue().bold(),
            output.to_string_lossy()
        );
    }

    Ok(())
}

/// Saves a "linked" problem (`LiftedProblem`) to a file.
///
/// This function performs the following steps:
/// 1. Ensures that all parent directories for the output file exist, creating them if necessary.
/// 2. Wraps the provided `LiftedProblem` in an `IRContent::LiftedProblem` container along with
///    the chosen serialization format.
/// 3. Creates an `Output` object to handle writing the content to disk.
/// 4. Writes the serialized problem to the specified output path.
/// 5. Resolves and prints the absolute path of the produced file for user confirmation.
///
/// # Type Parameters
/// - `P`: Any type that can be converted into a `PathBuf` (e.g., `&str`, `String`, `PathBuf`).
///
/// # Arguments
/// * `lifted_problem` - The lifted problem (linked problem) to save.
/// * `format` - The serialization format to use (JSON, etc.).
/// * `output_path` - Path where the serialized output should be written.
///
/// # Returns
/// * `Ok(())` if the file was successfully saved.
/// * `Err(IOError)` if any I/O operation (creating directories or writing the file) fails.
///
/// # Example
/// ```rust
/// use aiplan4rust::io::save_link_output;
/// use aiplan4rust::serialization::serde::SerdeFormat;
///
/// let lifted_problem = ...; // Construct or obtain a LiftedProblem
/// save_link_output(lifted_problem, SerdeFormat::Json, "output/problem.lifted")?;
/// ```
pub fn save_link_output<P: Into<PathBuf>>(
    lifted_problem: LiftedProblem,
    format: SerdeFormat,
    output_path: P,
) -> Result<(), IOError> {
    let output_path = output_path.into();

    // Create all parent directories if they do not exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Wrap the lifted problem in IRContent for output
    let content = IRContent::LiftedProblem(lifted_problem, format);
    let output = Output::new_ir(output_path.clone(), content);

    // Write the content to disk
    output.write()?;

    // Resolve the absolute path for display
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
