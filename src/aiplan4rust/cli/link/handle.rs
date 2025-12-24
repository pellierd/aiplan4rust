use std::path::{Path, PathBuf};
use clap::ArgMatches;
use colored::Colorize;
use crate::aiplan4rust::serialization::serde::{SerdeFormat, SerdeSerializable};
use crate::{Frontend, Renderer, Severity};
use crate::aiplan4rust::cli::cli::{FILES_ARG, FORMAT_ARG, OUTPUT_ARG};
use crate::aiplan4rust::cli::error::CliError;
use crate::aiplan4rust::cli::handle::ensure_parent_dir_exists;
use crate::aiplan4rust::io::{Extension, Output};
use crate::aiplan4rust::io::input::Input;
use crate::aiplan4rust::io::ir_kind::IRKind;
use crate::aiplan4rust::io::raw_kind::RawKind;


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

    // Vérifie qu'on a exactement deux fichiers : domain + problem
    if files.len() != 2 {
        return Err(CliError::InvalidFileCount);
    }

    let domain_file = PathBuf::from(&files[0]);
    let problem_file = PathBuf::from(&files[1]);

    // Récupère le format de sortie (default: JSON)
    let format = *matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();

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
    // Charge les sources
    let domain = Input::read_from_file(domain_file)?;
    let problem = Input::read_from_file(problem_file)?;

    match (&domain, &problem) {
        // ---------------------------------------------------------------------
        // Deux fichiers IR → link direct
        // ---------------------------------------------------------------------
        (
            Input::IR { content: domain_content, .. },
            Input::IR { content: problem_content, .. },
        ) => {
            // Vérifie que les types IR correspondent aux attentes
            if domain_content.kind() != IRKind::ParsedDomain
                || problem_content.kind() != IRKind::ParsedProblem
            {
                return Err(CliError::inconsistent_files());
            }

            link(&domain, &problem, format, &output_path)?;
        }
        // ---------------------------------------------------------------------
        // Deux fichiers Raw → parse + link
        // ---------------------------------------------------------------------
        (
            Input::Raw { content: domain_content, .. },
            Input::Raw { content: problem_content, .. },
        ) => {
            // Vérifie le rôle
            if domain_content.kind() != RawKind::Domain
                || problem_content.kind() != RawKind::Problem
            {
                return Err(CliError::inconsistent_files());
            }

            link_from_files(&domain, &problem, format, &output_path)?;
        }

        // ---------------------------------------------------------------------
        // Cas non supportés → erreur
        // ---------------------------------------------------------------------
        (
            Input::UnknownText { .. }
            | Input::BinaryUnknown { .. },
            _
        )
        | (
            _,
            Input::UnknownText { .. }
            | Input::BinaryUnknown { .. },
        ) => {
            return Err(CliError::inconsistent_files());
        }

        // ---------------------------------------------------------------------
        // Mix Raw / IR ou autres combinaisons invalides
        // ---------------------------------------------------------------------
        _ => {
            return Err(CliError::inconsistent_files());
        }
    }


    Ok(())
}


fn link(
    domain: &Input,
    problem: &Input,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    // Create a frontend instance
    let frontend = Frontend::new();

    // Perform linking, propagate any errors
    let linker_result = frontend.link(domain, problem)?;

    // If linking produced a semantic context, serialize it
    if let Some(planning_task) = linker_result.linked_semantic_context() {
        planning_task.serialize_to_file(format, output)?; // propagate SerializationError as CliError
        println!("Output saved to {}", output.to_string_lossy());
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
    domain: &Input,
    problem: &Input,
    format: SerdeFormat,
    output: &PathBuf,
) -> Result<(), CliError> {
    use std::time::Instant;

    let start_time = Instant::now();

    println!(
        "{:>10} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain.path().to_string_lossy(),
        problem.path().to_string_lossy()
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
            output.to_string_lossy()
        );
    }

    Ok(())
}
