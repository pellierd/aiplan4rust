use clap::{Arg, Command};
use crate::aiplan4rust::cli::parse::cli::build_parse_subcommand;
use crate::aiplan4rust::serialization::serde::SerdeFormat;

/// Application version.
pub const VERSION: &str = "1.0";
/// Application author.
pub const AUTHOR: &str = "Damien Pellier <damien.pellier@imag.fr>";
/// Short description of the application.
pub const ABOUT: &str = "aiplan";

/// Subcommand names.
pub const LINK_SUBCOMMAND: &str = "link";


/// Short and long flags for specifying a single output file.
pub const OUTPUT_SHORT: char = 'o'; // file
pub const OUTPUT_LONG: &str = "output";

/// Short and long flags for specifying an output directory.
pub const OUT_DIR_SHORT: char = 'd'; // directory
pub const OUT_DIR_LONG: &str = "out-dir";

pub const FORMAT_SHORT: char = 'f';
pub const FORMAT_LONG: &str = "format";


/// Output formats.
pub const JSON: &str = "json";
pub const YAML: &str = "yaml";
pub const TOML: &str = "toml";
pub const CBOR: &str = "cbor";
pub const MESSAGEPACK: &str = "messagepack";

// All supported output formats.
pub const SUPPORTED_FORMATS: &[&str] = &[
    JSON,
    YAML,
    TOML,
    CBOR,
    MESSAGEPACK,
];

/// Argument names.
pub const FILES_ARG: &str = "files";
pub const OUTPUT_ARG: &str = "output";
pub const FORMAT_ARG: &str = "format";
pub const OUT_DIR_ARG: &str = "out-dir";

pub const CURRENT_DIR: &str = ".";

// Texte d'aide pour chaque argument
pub const FILES_HELP: &str = "The domain and/or problem files to parse";
pub const OUTPUT_HELP: &str = "Output filename (single input file only)";
pub const OUT_DIR_HELP: &str = "Output directory for the output file(s) (default: current directory)";
pub const FORMAT_HELP: &str = "Output format (json, yaml, toml, cbor, messagepack)";

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
