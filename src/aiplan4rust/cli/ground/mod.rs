//! Module `ground`
//!
//! This module implements the `ground` subcommand for the `aiplan` CLI.
//!
//! # Overview
//!
//! The `ground` subcommand is responsible for grounding a linked planning problem
//! into a fully instantiated form. This module is split into two submodules:
//!
//! - [`handler`] — Contains the ops to execute the `ground` command, including
//!   grounding operations and any file manipulation required.
//! - [`cli`] — Defines the CLI interface, arguments, help messages, and constants
//!   for the `ground` subcommand.
//!
//! # Public API
//!
//! Only the following items are exposed to external modules:
//! - [`handle_ground_command`] — The main entry point for executing the `ground` command.
//! - [`GROUND_SUBCOMMAND`] — The constant name of the subcommand for CLI usage.
//!
//! # Example
//! ```rust
//! use aiplan4rust::cli::ground::{handle_ground_command, GROUND_SUBCOMMAND};
//!
//! // You can execute the command handler directly after parsing arguments
//! // handle_ground_command(&matches);
//! ```

pub mod handler;
pub mod cli;

/// Re-export the main handler function for the `ground` command.
pub use handler::handle_ground_command;

/// Re-export the subcommand name constant for `ground`.
pub use cli::GROUND_SUBCOMMAND;
