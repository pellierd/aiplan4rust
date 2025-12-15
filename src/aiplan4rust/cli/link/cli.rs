use crate::aiplan4rust::cli::cli::{
    FILES_ARG, FILES_HELP, FORMAT_ARG, FORMAT_HELP, FORMAT_LONG, FORMAT_SHORT, JSON, OUTPUT_ARG,
    OUTPUT_HELP, OUTPUT_LONG, OUTPUT_SHORT,
};
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use clap::{Arg, Command};

/// Subcommand names.
pub const LINK_SUBCOMMAND: &str = "link";

/// Short description of the `link` subcommand.
pub const LINK_ABOUT: &str = "Combine a domain and problem file into a combined output file";

/// Builds the `link` subcommand for the CLI.
pub fn build_link_subcommand() -> Command {
    Command::new(LINK_SUBCOMMAND)
        .about(LINK_ABOUT) // use the constant
        .arg(
            Arg::new(FILES_ARG)
                .help(FILES_HELP)
                .required(true)
                .num_args(2), // exactly two files: domain + problem
        )
        .arg(
            Arg::new(OUTPUT_ARG)
                .short(OUTPUT_SHORT)
                .long(OUTPUT_LONG)
                .help(OUTPUT_HELP)
                .value_parser(clap::value_parser!(String))
                .required(true),
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
