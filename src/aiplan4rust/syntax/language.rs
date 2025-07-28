//! This module defines the `Language` enum and related constants used to
//! identify and parse different syntax languages.
//!
//! Planning languages specify the syntax and semantics for describing syntax
//! domains and problems in automated syntax systems.
//!
//! Currently supported languages:
//! - PDDL (Planning Domain Definition Language): The classical language for
//!   expressing syntax domains and problems.
//! - HDDL (Hierarchical Domain Definition Language): An extension of PDDL
//!   that supports hierarchical task structures for HTN syntax.
//!
//! The `Language` enum supports parsing from string identifiers such as
//! `"pddl"` and `"hddl"` (case-insensitive).
//!
//! # Examples
//!
//! ```rust
//! use aiplan4rust::parser::Language;
//! use std::str::FromStr;
//!
//! let lang: Language = "hddl".parse().unwrap();
//! assert_eq!(lang, Language::HDDL);
//! ```
//!
//! # Constants
//! - [`PDDL_LANGUAGE`]: The string literal `"pddl"` used as a language identifier.
//! - [`HDDL_LANGUAGE`]: The string literal `"hddl"` used as a language identifier.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Constant representing the "pddl" language identifier (used in parsing).
pub const PDDL_LANGUAGE: &str = "pddl";

/// Constant representing the "hddl" language identifier (used in parsing).
pub const HDDL_LANGUAGE: &str = "hddl";

/// An enum representing different types of syntax languages.
///
/// This enum is used to identify the expected language of the domain/problem being parsed.
/// It currently supports:
/// - [`Language::PDDL`] — the classical Planning Domain Definition Language,
/// - [`Language::HDDL`] — an extension of PDDL that introduces hierarchical task structures.
///
/// # Variants
/// - `PDDL`: The standard Planning Domain Definition Language.
/// - `HDDL`: The Hierarchical Domain Definition Language (extension of PDDL).
///
/// # Default
/// The default language is [`Language::PDDL`].
///
/// # Example
/// ```rust
/// use aiplan4rust::parser::Language;
/// use std::str::FromStr;
///
/// let lang: Language = "hddl".parse().unwrap();
/// assert_eq!(lang, Language::HDDL);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    PDDL,
    HDDL,
}

impl Default for Language {
    /// Returns [`Language::PDDL`] as the default language.
    fn default() -> Self {
        Language::PDDL
    }
}

impl FromStr for Language {
    type Err = String;

    /// Converts a string to a [`Language`] enum.
    ///
    /// Accepts lowercase identifiers `"pddl"` and `"hddl"` (case-insensitive).
    ///
    /// # Errors
    /// Returns an error if the input string doesn't match any known language.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            PDDL_LANGUAGE => Ok(Language::PDDL),
            HDDL_LANGUAGE => Ok(Language::HDDL),
            _ => Err(format!("Invalid language: '{}'", s)),
        }
    }
}
