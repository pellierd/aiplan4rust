use crate::aiplan4rust::cli::cli::{
    CURRENT_DIR,
    FILES_ARG,
    FILES_HELP,
    FORMAT_ARG,
    FORMAT_HELP,
    FORMAT_LONG,
    FORMAT_SHORT,
    JSON,
    OUTPUT_ARG,
    OUTPUT_HELP,
    OUTPUT_LONG,
    OUTPUT_SHORT,
    OUT_DIR_ARG,
    OUT_DIR_HELP,
    OUT_DIR_LONG,
    OUT_DIR_SHORT,
};
use crate::aiplan4rust::serialization::serde::SerdeFormat;
use clap::{Arg, Command};

/// Subcommand names.
pub const GROUND_SUBCOMMAND: &str = "ground";

/// Short description of the `ground` subcommand.
pub const GROUND_ABOUT: &str = "Ground a planning problem into a fully instantiated form";

/// Builds the `ground` subcommand for the CLI.
pub fn build_ground_subcommand() -> Command {
    Command::new(GROUND_SUBCOMMAND)
        .about(GROUND_ABOUT)
        .arg(
            Arg::new(FILES_ARG)
                .help(FILES_HELP)
                .required(true)
                .num_args(1..), // typically one linked file
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
