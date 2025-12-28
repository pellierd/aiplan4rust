//! Module `parse`
//!
//! This module implements the `parse` subcommand for the `aiplan` CLI.
//!
//! It is split into two submodules:
//! - [`cli`]: defines the CLI interface, arguments, and help messages for the `parse` subcommand.
//! - [`handler`]: contains the logic for executing the `parse` command, including file parsing
//!   and output generation.
//!
//! # Public API
//!
//! - [`handle_parse_command`] — Entry point for handling the `parse` command after argument parsing.
//!
//! # Example
//! ```rust
//! use aiplan4rust::cli::parse::{handle_parse_command, cli::build_parse_subcommand};
//!
//! // Build the CLI subcommand
//! let parse_cmd = build_parse_subcommand();
//!
//! // Later, after getting matches from clap:
//! // handle_parse_command(&matches);
//! ```

pub mod cli;
pub mod handler;

/// Re-export the main handler function for the `parse` command.
pub use handler::handle_parse_command;
pub use cli::PARSE_SUBCOMMAND;
