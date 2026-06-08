use crate::aiplan4rust::cli::cli::{
    CURRENT_DIR, FILES_ARG, FILES_HELP, FORMAT_ARG, FORMAT_HELP, FORMAT_LONG, FORMAT_SHORT, JSON,
    OUTPUT_ARG, OUTPUT_HELP, OUTPUT_LONG, OUTPUT_SHORT, OUT_DIR_ARG, OUT_DIR_HELP, OUT_DIR_LONG,
    OUT_DIR_SHORT,
};
use crate::aiplan4rust::cli::io::serialization::serde::SerdeFormat;
use clap::{Arg, Command};

/// Subcommand names.
pub const LINK_SUBCOMMAND: &str = "link";

/// Short description of the `link` subcommand.
pub const LINK_ABOUT: &str = "Combine a domain and problem file into a combined output file";

/// Builds the `link` subcommand for the CLI.
pub fn build_link_subcommand() -> Command {
    Command::new(LINK_SUBCOMMAND)
        .about(LINK_ABOUT)
        .arg(
            Arg::new(FILES_ARG)
                .help(FILES_HELP)
                .required(true)
                .num_args(2..), // domain + problems
        )
        .arg(
            Arg::new(OUTPUT_ARG)
                .short(OUTPUT_SHORT)
                .long(OUTPUT_LONG)
                .help(OUTPUT_HELP)
                .value_parser(clap::value_parser!(String))
                .required(false),
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
