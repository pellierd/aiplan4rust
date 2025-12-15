//! CLI constants and main command builder for the `aiplan` application.
//!
//! This module defines:
//! 1. Application metadata (`AUTHOR`, `ABOUT`).
//! 2. CLI argument names, flags, help texts, and supported output formats.
//! 3. The `build_cli` function, which constructs the top-level `aiplan` CLI command
//!    including all subcommands (`link` and `parse`).

use clap::Command;
use crate::aiplan4rust::cli::link::cli::build_link_subcommand;
use crate::aiplan4rust::cli::parse::cli::build_parse_subcommand;

/// Application author.
pub const AUTHOR: &str = "Damien Pellier <damien.pellier@imag.fr>";

/// Short description of the application.
pub const ABOUT: &str = "aiplan";

/// Short and long flags for specifying a single output file.
pub const OUTPUT_SHORT: char = 'o';
pub const OUTPUT_LONG: &str = "output";

/// Short and long flags for specifying an output directory.
pub const OUT_DIR_SHORT: char = 'd';
pub const OUT_DIR_LONG: &str = "out-dir";

/// Short and long flags for specifying the output format.
pub const FORMAT_SHORT: char = 'f';
pub const FORMAT_LONG: &str = "format";

/// Supported output formats.
pub const JSON: &str = "json";
pub const YAML: &str = "yaml";
pub const TOML: &str = "toml";
pub const CBOR: &str = "cbor";
pub const MESSAGEPACK: &str = "messagepack";

/// All supported output formats.
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

/// Current directory constant.
pub const CURRENT_DIR: &str = ".";

/// Help messages for CLI arguments.
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
///
/// # Example
/// ```rust
/// use aiplan4rust::cli::aiplan_cli::build_cli;
///
/// let cli = build_cli();
/// let matches = cli.get_matches();
/// ```
pub fn build_cli() -> Command {
    Command::new("aiplan")
        .version(env!("CARGO_PKG_VERSION"))
        .author(AUTHOR)
        .about(ABOUT)
        .arg_required_else_help(true)
        .subcommand(build_link_subcommand())
        .subcommand(build_parse_subcommand())
}
