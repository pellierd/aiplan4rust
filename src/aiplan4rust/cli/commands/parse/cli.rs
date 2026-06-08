//! Module `parse`
//!
//! This module defines the CLI subcommand for parsing PDDL or HDDL domain/problem files.
//! It provides constants for the subcommand name, descriptions, and argument identifiers
//! used by `clap` when building the CLI.
//!
//! # Example
//! ```rust
//! use crate::aiplan4rust::cli::parse::*;
//! let parse_cmd = build_parse_subcommand();
//! ```

use crate::aiplan4rust::cli::cli::{
    CURRENT_DIR, FILES_ARG, FILES_HELP, FORMAT_ARG, FORMAT_HELP, FORMAT_LONG, FORMAT_SHORT, JSON,
    OUTPUT_ARG, OUTPUT_HELP, OUTPUT_LONG, OUTPUT_SHORT, OUT_DIR_ARG, OUT_DIR_HELP, OUT_DIR_LONG,
    OUT_DIR_SHORT,
};
use crate::aiplan4rust::cli::io::serialization::serde::SerdeFormat;
use clap::{Arg, Command};

/// Name of the `parse` subcommand used in the CLI.
pub const PARSE_SUBCOMMAND: &str = "parse";

/// Short description of the `parse` subcommand, displayed in CLI help.
pub const PARSE_ABOUT: &str =
    "Parse one or more PDDL or HDDL domain/problem files and generate output files";

/// Builds the `parse` subcommand for the CLI.
///
/// This subcommand allows parsing one or more PDDL or HDDL domain/problem files
/// and optionally generating output files in a specified serialization format.
///
/// # Arguments
/// This subcommand accepts the following CLI arguments:
/// - `<files>` (required, 1 or more): domain and/or problem files to parse.
/// - `-o, --output` (optional, single file only): output filename if only one input file is provided.
/// - `-d, --out-dir` (optional, multiple files only): directory where output files will be written (debug: current directory).
/// - `-f, --format` (optional, debug: `json`): output serialization format. Possible values: `json`, `yaml`, `toml`, `cbor`, `messagepack`.
///
/// # Rules
/// - If a single input file is provided:
///     - Use `-o/--output` to specify the output filename.
///     - If omitted, the filename is inferred automatically.
/// - If multiple input files are provided:
///     - `-o/--output` is forbidden.
///     - `-d/--out-dir` is required; the directory will be created if it does not exist.
///
/// # Returns
/// Returns a [`Command`] configured as the `parse` subcommand, ready to be added
/// to the top-level CLI.
///
/// # Example
/// ```rust
/// let parse_cmd = build_parse_subcommand();
/// ```
pub fn build_parse_subcommand() -> Command {
    Command::new(PARSE_SUBCOMMAND)
        .about(PARSE_ABOUT)
        .arg(
            Arg::new(FILES_ARG)
                .help(FILES_HELP)
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new(OUTPUT_ARG)
                .short(OUTPUT_SHORT)
                .long(OUTPUT_LONG)
                .help(OUTPUT_HELP)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new(OUT_DIR_ARG)
                .short(OUT_DIR_SHORT)
                .long(OUT_DIR_LONG)
                .help(OUT_DIR_HELP)
                .default_value(CURRENT_DIR),
        )
        .arg(
            Arg::new(FORMAT_ARG)
                .short(FORMAT_SHORT)
                .long(FORMAT_LONG)
                .help(FORMAT_HELP)
                .value_parser(clap::value_parser!(SerdeFormat))
                .default_value(JSON),
        )
}
