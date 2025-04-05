use crate::aiplan4rust::file_format::FileFormat;
use crate::aiplan4rust::syntax::Language;
use clap::{Arg, Command};

/// Constants for command names and other strings
pub const VERSION: &str = "1.0";
pub const AUTHOR: &str = "Damien Pellier <damien.pellier@imag.fr>";
pub const ABOUT: &str = "aiplan";
pub const LINK_SUBCOMMAND: &str = "link";
pub const PARSE_SUBCOMMAND: &str = "parse";
pub const OUTPUT_SHORT: char = 'o';
pub const OUTPUT_LONG: &str = "output";
pub const FORMAT_SHORT: char = 'f';
pub const FORMAT_LONG: &str = "format";
pub const LANGUAGE_SHORT: char = 'l';
pub const LANGUAGE_LONG: &str = "language";
pub const JSON: &str = "json";
pub const YAML: &str = "yaml";
pub const FILES_ARG: &str = "files";
pub const OUTPUT_ARG: &str = "output";
pub const FORMAT_ARG: &str = "format";
pub const LANGUAGE_ARG: &str = "language";

/// Builds the main CLI command for the `aiplan` application.
///
/// This function initializes the command-line interface, specifying metadata
/// such as version, author, and description, and registers subcommands.
pub fn build_cli() -> Command {
    Command::new("aiplan")
        .version(VERSION)
        .author(AUTHOR)
        .about(ABOUT)
        .subcommand(build_link_subcommand())
        .subcommand(build_parse_subcommand())
}

/// Builds the `link` subcommand.
///
/// This subcommand combines a domain and problem file into a single output file.
///
/// # Arguments:
/// - `files`: The domain and problem files (required, exactly 2 files).
/// - `output`: The name of the output file (required).
/// - `format`: The output format, either JSON or YAML (default: JSON).
/// - `language`: The language for parsing, either PDDL (default) or HDDL.
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
                .value_parser(clap::value_parser!(FileFormat))
                .default_value(JSON),
        )
        .arg(
            Arg::new(LANGUAGE_ARG)
                .short(LANGUAGE_SHORT)
                .long(LANGUAGE_LONG)
                .help("Defines the language for parsing (pddl or hddl)")
                .value_parser(clap::value_parser!(Language))
                .default_value("PDDL"),
        )
}

/// Builds the `parse` subcommand.
///
/// This subcommand parses a PDDL domain and/or problem file and generates an output file.
///
/// # Arguments:
/// - `files`: The domain and/or problem files (required, 1 or 2 files).
/// - `output`: The name of the output file (optional).
/// - `format`: The output format, either JSON or YAML (default: JSON).
/// - `language`: The language for parsing, either PDDL (default) or HDDL.
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
                .value_parser(clap::value_parser!(FileFormat))
                .default_value(JSON),
        )
        .arg(
            Arg::new(LANGUAGE_ARG)
                .short(LANGUAGE_SHORT) // Utilisation de la constante 'l'
                .long(LANGUAGE_LONG) // Utilisation de la constante 'language'
                .help("Defines the language for parsing (pddl or hddl)")
                .value_parser(clap::value_parser!(Language))
                .default_value("PDDL"),
        )
}
