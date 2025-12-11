use aiplan4rust::aiplan4rust::cli::aiplan_cli::{
    build_cli, FILES_ARG, FORMAT_ARG, LINK_SUBCOMMAND, OUTPUT_ARG, PARSE_SUBCOMMAND,
};
use aiplan4rust::aiplan4rust::serialization::serde::{SerdeExtension, SerdeFormat};
use aiplan4rust::aiplan4rust::Frontend;
use aiplan4rust::aiplan4rust::diagnostic::{Renderer, Severity};
use aiplan4rust::aiplan4rust::serialization::serde::SerdeSerializable;

use clap::ArgMatches;
use std::path::Path;
use std::time::Instant;
use colored::Colorize;


/// Handles the `link` command logic.
///
/// This function retrieves the domain and problem files from the CLI arguments,
/// validates that exactly two files are provided, and calls the `link` function
/// to combine them into the specified output format.
///
/// # Arguments:
/// - `matches`: Parsed command-line arguments for the `link` subcommand.
fn handle_link_command(matches: &ArgMatches) {
    if let Some(files) = matches.get_many::<String>(FILES_ARG) {
        let files_vec: Vec<String> = files.cloned().collect();
        if files_vec.len() == 2 {
            let output = matches.get_one::<String>(OUTPUT_ARG).unwrap();
            let format = matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();
            link(&files_vec[0], &files_vec[1], *format, output);
        } else {
            eprintln!("Error: You must provide exactly two files (domain and problem).")
        }
    }
}

/// Handles the `parse` command logic.
///
/// This function retrieves the domain and/or problem files from the CLI arguments,
/// determines whether one or two files are provided, and calls the appropriate parsing function.
/// If an output file is not specified, a default name is generated.
///
/// # Arguments:
/// - `matches`: Parsed command-line arguments for the `parse` subcommand.
fn handle_parse_command(matches: &ArgMatches) {
    if let Some(files) = matches.get_many::<String>(FILES_ARG) {
        let files_vec: Vec<String> = files.cloned().collect();
        let format = *matches.get_one::<SerdeFormat>(FORMAT_ARG).unwrap();

        match files_vec.len() {
            1 => {
                let input_file = &files_vec[0];
                let output = matches
                    .get_one::<String>(OUTPUT_ARG)
                    .cloned()
                    .unwrap_or_else(|| {
                        generate_lifted_domain_or_problem_filename(input_file, format)
                    });
                parse_file(input_file, format, &output);
            }
            2 => {
                let domain_file = &files_vec[0];
                let problem_file = &files_vec[1];
                let output = matches
                    .get_one::<String>(OUTPUT_ARG)
                    .cloned()
                    .unwrap_or_else(|| {
                        generate_lifted_planning_task_filename(domain_file, problem_file, format)
                    });
                parse(domain_file, problem_file, format, &output);
            }
            _ => {
                eprintln!("Error: You must provide one or two files.");
            }
        }
    }
}

/// Main entry point for the application.
///
/// Parses command-line arguments and dispatches execution to the appropriate command handler.
fn main() {
    env_logger::init();

    let matches = build_cli().get_matches();

    if let Some(matches) = matches.subcommand_matches(LINK_SUBCOMMAND) {
        handle_link_command(matches);
    } else if let Some(matches) = matches.subcommand_matches(PARSE_SUBCOMMAND) {
        handle_parse_command(matches);
    }
}

fn link(domain_file: &str, problem_file: &str, format: SerdeFormat, output: &str) {
    let frontend = Frontend::new();

    // Appel de la méthode link sur frontend
    match frontend.link(domain_file, problem_file) {
        Ok(linker_result) => {
            if let Some(planning_task) = linker_result.linked_semantic_context() {
                if let Err(e) =
                    planning_task.serialize_to_file(format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!("Output saved to {}", output);
                }
            } else {
                let mut renderer = Renderer::new(linker_result.diagnostic_manager(), linker_result.interner());
                let _ =  renderer.display();
            }
        }
        Err(e) => {
            eprintln!("Error during linking: {}", e);
        }
    }
}

pub fn parse(
    domain_file: &str,
    problem_file: &str,
    format: SerdeFormat,
    output: &str,
) {
    let start_time = Instant::now();

    println!(
        "{:>10} aiplan4rust v0.1.0 (domain: {}, problem: {})",
        "Parsing".green().bold(),
        domain_file,
        problem_file
    );

    let frontend = Frontend::new();
    match frontend.parse(domain_file, problem_file) {
        Ok(result) => {
            let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner());
            let _ = renderer.display();
            

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
                if let Err(e) =
                    lifted_problem.serialize_to_file(format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!(
                        "{} Output saved to {}",
                        "===>".blue().bold(),
                        output
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn parse_file(input_file: &str, format: SerdeFormat, output: &str) {
    let start_time = Instant::now(); // Démarre le chronomètre

    let full_path = Path::new(input_file)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| input_file.to_string());

    println!(
        "{:>10} aiplan4rust v0.1.0 ({})",
        "Parsing".green().bold(),
        full_path
    );

    let frontend = Frontend::new();
    match frontend.parse_file(input_file) {
        Ok(result) => {
            let mut renderer = Renderer::new(result.diagnostic_manager(), result.interner()) ;
            let _ = renderer.display();

            // Compte les erreurs et les warnings
            let error_count = result.diagnostic_manager().count_diagnostics_of_severity(Severity::Error);
            let warning_count = result.diagnostic_manager().count_diagnostics_of_severity(Severity::Warning);

            // Chronomètre
            let elapsed_time = start_time.elapsed().as_secs_f32();

            // Affichage du message de fin
            println!(
                "{} {} error(s), {} warning(s) target(s) in {:.2}s",
                "Finished".green().bold(),
                format!("{}", error_count),
                format!("{}", warning_count),
                elapsed_time
            );

            // Si des erreurs sont présentes, indiquer qu'aucun fichier n'a été produit
            if error_count > 0 {
                println!(
                    "{} No output file produced due to errors.",
                    "===> ".blue().bold());
            } else {
                // Si aucun problème, afficher que le fichier a été produit
                if let Some(context) = result.semantic_context() {
                    if let Err(e) = context.serialize_to_file(format, output) {
                        eprintln!("Error saving file: {}", e);
                    } else {
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
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}


/// Generates an output file name based on the input file and the specified format.
///
/// # Parameters
/// - `input_file`: A string slice representing the input file path.
/// - `format`: A reference to a `FileFormat` enumeration that specifies the desired output file format.
///
/// # Returns
/// A string representing the output file name with the same base name as the input file, but with
/// the extension corresponding to the specified format.
///
/// # Example
/// ```rust
/// let filename = generate_lifted_domain_or_problem_filename("example.pddl", &FileFormat::Json);
/// assert_eq!(filename, "example.json");
/// ```
///
/// # Notes
/// If the input file doesn't have an extension, it will be used as is as the base name.
fn generate_lifted_domain_or_problem_filename(input_file: &str, format: SerdeFormat) -> String {
    let base_name = input_file
        .rsplit_once('.')
        .map(|(name, _ext)| name)
        .unwrap_or(input_file);
    let extension = SerdeExtension::from(format).as_str();
    format!("{}.{}", base_name, extension)
}

/// Generates an output file name based on the domain and problem file names and the specified format.
///
/// # Parameters
/// - `domain_file`: A string slice representing the domain file path.
/// - `problem_file`: A string slice representing the problem file path.
/// - `format`: A reference to a `Format` enumeration that specifies the desired output file format.
///
/// # Returns
/// A string representing the output file name, which is a combination of the base names of the
/// domain and problem files, separated by an underscore, with the extension corresponding to the
/// specified format.
///
/// # Example
/// ```rust
/// let filename = generate_lifted_planning_task_filename("domain.pddl", "problem.pddl", &Format::Xml);
/// assert_eq!(filename, "problem_domain.xml");
/// ```
///
/// # Notes
/// If either the domain or problem file does not have a valid name (i.e., no extension or invalid file),
/// the function defaults to `"unknown_domain"` and `"unknown_problem"` respectively.
fn generate_lifted_planning_task_filename(
    domain_file: &str,
    problem_file: &str,
    format: SerdeFormat,
) -> String {
    let domain_stem = Path::new(domain_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_domain");

    let problem_stem = Path::new(problem_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_problem");
    let extension = SerdeExtension::from(format).as_str();
    format!("{}_{}.{}", problem_stem, domain_stem, extension)
}
