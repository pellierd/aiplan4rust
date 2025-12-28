//! This module defines the `Language` type, representing the language
//! used for raw content in planning systems.
//!
//! # Description
//!
//! The module provides the `Language` enum with two variants:
//! - `PDDL` for **Planning Domain Definition Language**
//! - `HDDL` for **Hierarchical Domain Definition Language**
//!
//! It also includes a `fmt::Display` implementation for obtaining
//! a human-readable string representation.
//!
//! # Example
//!
//! ```rust
//! use your_crate::Language;
//!
//! let lang = Language::HDDL;
//! println!("The selected language is {}", lang);
//! ```

use std::fmt;

/// Represents the language of raw content.
///
/// # Variants
///
/// - `PDDL`: Planning Domain Definition Language
/// - `HDDL`: Hierarchical Domain Definition Language
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Language {
    /// Planning Domain Definition Language
    PDDL,
    /// Hierarchical Domain Definition Language
    HDDL,
}

impl fmt::Display for Language {
    /// Formats the `Language` enum as a human-readable string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use your_crate::Language;
    ///
    /// let lang = Language::PDDL;
    /// assert_eq!(format!("{}", lang), "PDDL");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::PDDL => write!(f, "PDDL"),
            Language::HDDL => write!(f, "HDDL"),
        }
    }
}
