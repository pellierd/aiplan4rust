//! Module `link`
//!
//! This module implements the `link` subcommand for the `aiplan` CLI.
//!
//! # Overview
//!
//! The `link` subcommand is responsible for performing linking operations within
//! the AI planning workflow. This module is split into two submodules:
//!
//! - [`handler`] — Contains the ops to execute the `link` command, including
//!   any processing or file manipulation required.
//! - [`cli`] — Defines the CLI interface, arguments, help messages, and constants
//!   for the `link` subcommand.
//!
//! # Public API
//!
//! Only the following items are exposed to external modules:
//! - [`handle_link_command`] — The main entry point for executing the `link` command.
//! - [`LINK_SUBCOMMAND`] — The constant name of the subcommand for CLI usage.
//!
//! # Example
//! ```rust
//! use aiplan4rust::cli::link::{handle_link_command, LINK_SUBCOMMAND};
//!
//! // You can execute the command handler directly after parsing arguments
//! // handle_link_command(&matches);
//! ```

pub mod handler;
pub mod cli;

/// Re-export the main handler function for the `link` command.
pub use handler::handle_link_command;

/// Re-export the subcommand name constant for `link`.
pub use cli::LINK_SUBCOMMAND;
