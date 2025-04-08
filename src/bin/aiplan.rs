use aiplan4rust::aiplan4rust::cli::aiplan_cli::{
    build_cli, FILES_ARG, FORMAT_ARG, LANGUAGE_ARG, LINK_SUBCOMMAND, OUTPUT_ARG, PARSE_SUBCOMMAND,
};
use aiplan4rust::aiplan4rust::parser::Language;
use aiplan4rust::aiplan4rust::FileFormat;
use aiplan4rust::aiplan4rust::Frontend;

use clap::ArgMatches;
use std::path::Path;

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
            let format = matches.get_one::<FileFormat>(FORMAT_ARG).unwrap();
            link(&files_vec[0], &files_vec[1], format, output);
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
    // Retrieve the language argument
    let language = matches
        .get_one::<Language>(LANGUAGE_ARG)
        .unwrap_or(&Language::PDDL); // Default to PDDL if not provided

    if let Some(files) = matches.get_many::<String>(FILES_ARG) {
        let files_vec: Vec<String> = files.cloned().collect();
        let format = matches.get_one::<FileFormat>(FORMAT_ARG).unwrap();

        match files_vec.len() {
            1 => {
                let input_file = &files_vec[0];
                let output = matches
                    .get_one::<String>(OUTPUT_ARG)
                    .cloned()
                    .unwrap_or_else(|| {
                        generate_lifted_domain_or_problem_filename(input_file, format)
                    });
                parse_file(input_file, &language, &format, &output);
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
                parse(domain_file, problem_file, &language, &format, &output);
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
    let matches = build_cli().get_matches();

    if let Some(matches) = matches.subcommand_matches(LINK_SUBCOMMAND) {
        handle_link_command(matches);
    } else if let Some(matches) = matches.subcommand_matches(PARSE_SUBCOMMAND) {
        handle_parse_command(matches);
    }
}

fn link(domain_file: &str, problem_file: &str, format: &FileFormat, output: &str) {
    let frontend = Frontend::new();

    // Appel de la méthode link sur frontend
    match frontend.link(domain_file, problem_file) {
        Ok(linker_result) => {
            if let Some(planning_task) = linker_result.planning_task() {
                // Si le linking réussit et qu'il y a un planning_task, le sérialiser
                if let Err(e) =
                    frontend.serialize_planning_task_to_file(&planning_task, format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!("Output saved to {}", output);
                }
            } else {
                linker_result.error_manager().display_all();
            }
        }
        Err(e) => {
            eprintln!("Error during linking: {}", e);
        }
    }
}

fn parse(
    domain_file: &str,
    problem_file: &str,
    language: &Language,
    format: &FileFormat,
    output: &str,
) {
    let frontend = Frontend::new();
    match frontend.parse(domain_file, problem_file, language) {
        Ok(result) => {
            result.error_manager().display_all();
            if let Some(planning_task) = result.planning_task() {
                if let Err(e) =
                    frontend.serialize_planning_task_to_file(&planning_task, format, output)
                {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!("Output saved to {}", output);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

fn parse_file(input_file: &str, language: &Language, format: &FileFormat, output: &str) {
    let frontend = Frontend::new();
    match frontend.parse_file(input_file, language) {
        Ok(result) => {
            result.error_manager().display_all();
            if let Some(ast) = result.annotated_syntax_tree() {
                if let Err(e) = frontend.serialize_to_file(&ast, format, output) {
                    eprintln!("Error saving file: {}", e);
                } else {
                    println!("Output saved to {}", output);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
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
fn generate_lifted_domain_or_problem_filename(input_file: &str, format: &FileFormat) -> String {
    let base_name = input_file
        .rsplit_once('.')
        .map(|(name, _ext)| name)
        .unwrap_or(input_file);

    format!("{}.{}", base_name, format.extension())
}

/// Generates an output file name based on the domain and problem file names and the specified format.
///
/// # Parameters
/// - `domain_file`: A string slice representing the domain file path.
/// - `problem_file`: A string slice representing the problem file path.
/// - `format`: A reference to a `FileFormat` enumeration that specifies the desired output file format.
///
/// # Returns
/// A string representing the output file name, which is a combination of the base names of the
/// domain and problem files, separated by an underscore, with the extension corresponding to the
/// specified format.
///
/// # Example
/// ```rust
/// let filename = generate_lifted_planning_task_filename("domain.pddl", "problem.pddl", &FileFormat::Xml);
/// assert_eq!(filename, "problem_domain.xml");
/// ```
///
/// # Notes
/// If either the domain or problem file does not have a valid name (i.e., no extension or invalid file),
/// the function defaults to `"unknown_domain"` and `"unknown_problem"` respectively.
fn generate_lifted_planning_task_filename(
    domain_file: &str,
    problem_file: &str,
    format: &FileFormat,
) -> String {
    let domain_stem = Path::new(domain_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_domain");

    let problem_stem = Path::new(problem_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_problem");

    format!("{}_{}.{}", problem_stem, domain_stem, format.extension())
}
