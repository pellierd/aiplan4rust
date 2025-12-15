
use aiplan4rust::aiplan4rust::cli::build_cli;
use aiplan4rust::aiplan4rust::serialization::serde::{SerdeExtension, SerdeFormat};
use std::path::{Path};
use aiplan4rust::aiplan4rust::cli::link::{handle_link_command, LINK_SUBCOMMAND};
use aiplan4rust::aiplan4rust::cli::parse::cli::PARSE_SUBCOMMAND;
use aiplan4rust::aiplan4rust::cli::parse::handle_parse_command;




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
