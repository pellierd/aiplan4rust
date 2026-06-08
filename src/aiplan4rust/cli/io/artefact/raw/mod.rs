//! Submodules for raw planning files.
//!
//! This module exposes internal components related to raw planning files (PDDL/HDDL).
//! It is intended for internal use within the `artefact::raw` module, hence the `pub(super)` visibility.
//!
//! # Submodules
//!
//! - [`content`]: Defines the `RawContent` struct and related methods for representing
//!   the textual content of raw domain and problem files, along with their typing and language.
//!
//! - [`kind`]: Defines the `RawKind` enum used to distinguish between domain and problem files.

/// Manages the textual content of raw planning files (domains or problems).
///
/// Contains the `RawContent` struct which stores:
/// - The kind of file (`Domain` or `Problem`),
/// - The planning language (PDDL, HDDL, etc.),
/// - The actual textual content.
///
/// This module provides constructors, accessors, and formatting utilities.
pub(in crate::aiplan4rust) mod content;

/// Defines the typing of raw planning file.
///
/// `RawKind` distinguishes between domain and problem files, enabling
/// the pipeline to correctly handle parsing, linking, and validation.
pub(in crate::aiplan4rust) mod kind;
