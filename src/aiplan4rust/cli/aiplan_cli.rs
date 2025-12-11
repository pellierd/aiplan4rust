use clap::{Arg, Command};
use crate::aiplan4rust::serialization::serde::SerdeFormat;

/// Application version.
pub const VERSION: &str = "1.0";
/// Application author.
pub const AUTHOR: &str = "Damien Pellier <damien.pellier@imag.fr>";
/// Short description of the application.
pub const ABOUT: &str = "aiplan";

/// Subcommand names.
pub const LINK_SUBCOMMAND: &str = "link";
pub const PARSE_SUBCOMMAND: &str = "parse";

/// Argument short and long flags.
pub const OUTPUT_SHORT: char = 'o';
pub const OUTPUT_LONG: &str = "output";
pub const FORMAT_SHORT: char = 'f';
pub const FORMAT_LONG: &str = "format";
pub const LANGUAGE_SHORT: char = 'l';
pub const LANGUAGE_LONG: &str = "language";

/// Output formats.
pub const JSON: &str = "json";
pub const YAML: &str = "yaml";

/// Argument names.
pub const FILES_ARG: &str = "files";
pub const OUTPUT_ARG: &str = "output";
pub const FORMAT_ARG: &str = "format";

/// Builds the main CLI command for the `aiplan` application.
///
/// This function sets the application's metadata (version, author, description)
/// and registers the subcommands `link` and `parse`.
///
/// # Returns
/// A [`Command`] ready to be used with `.get_matches()`.
pub fn build_cli() -> Command {
    Command::new("aiplan")
        .version(VERSION)
        .author(AUTHOR)
        .about(ABOUT)
        .arg_required_else_help(true)
        .subcommand(build_link_subcommand())
        .subcommand(build_parse_subcommand())
}

/// Builds the `link` subcommand.
///
/// The `link` subcommand combines a domain and problem file into a single output file.
///
/// # Arguments
/// - `files` (required, exactly 2): domain and problem files
/// - `output` (required): output file name
/// - `format` (optional, default `json`): output format (`json` or `yaml`)
///
/// # Returns
/// A [`Command`] representing the `link` subcommand.
pub fn build_link_subcommand() -> Command {
    Command::new(LINK_SUBCOMMAND)
        .about("Combine a domain and problem file into a combined output file")
        .arg(
            Arg::new(FILES_ARG)
                .help("The domain and problem files")
                .required(true)
                .num_args(2),
        )
        .arg(
            Arg::new(OUTPUT_ARG)
                .short(OUTPUT_SHORT)
                .long(OUTPUT_LONG)
                .help("Output file name for the combined result (e.g., domain_problem.json)")
                .value_parser(clap::value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new(FORMAT_ARG)
                .short(FORMAT_SHORT)
                .long(FORMAT_LONG)
                .help("Defines the output format (json or yaml)")
                .value_parser(clap::value_parser!(SerdeFormat))
                .default_value(JSON),
        )
}

/// Builds the `parse` subcommand.
///
/// The `parse` subcommand parses a PDDL or HDDL domain and/or problem file and
/// optionally generates an output file in the specified format.
///
/// # Arguments
/// - `files` (required, 1 or 2): domain and/or problem files
/// - `output` (optional): output file name
/// - `format` (optional, default `json`): output format (`json` or `yaml`)
///
/// # Returns
/// A [`Command`] representing the `parse` subcommand.
pub fn build_parse_subcommand() -> Command {
    Command::new(PARSE_SUBCOMMAND)
        .about("Parse a PDDL domain and/or problem file, and generate an output file")
        .arg(
            Arg::new(FILES_ARG)
                .help("The domain and/or problem files")
                .required(true)
                .num_args(1..=2),
        )
        .arg(
            Arg::new(OUTPUT_ARG)
                .short(OUTPUT_SHORT)
                .long(OUTPUT_LONG)
                .help("Specify the output file name")
                .value_parser(clap::value_parser!(String))
                .required(false),
        )
        .arg(
            Arg::new(FORMAT_ARG)
                .short(FORMAT_SHORT)
                .long(FORMAT_LONG)
                .help("Defines the output format (json or yaml)")
                .value_parser(clap::value_parser!(SerdeFormat))
                .default_value(JSON),
        )
}
