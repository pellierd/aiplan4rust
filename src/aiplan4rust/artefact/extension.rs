//! This module defines the `Extension` enum, representing the different
//! file extensions used in the planning system workflow.
//!
//! # Description
//!
//! The enum covers both raw input languages and intermediate pipeline artifacts:
//! - Raw input: `Pddl`, `Hddl`
//! - Pipeline artifacts: `Parsed` (`.prs`), `Lifted` (`.lft`), `Ground` (`.grd`)
//!
//! It also provides methods to get the string representation of each variant
//! and a `Display` implementation for pretty printing.
//!
//! # Example
//!
//! ```rust
//! use your_crate::Extension;
//!
//! let ext = Extension::Lifted;
//! println!("File extension: {}", ext); // prints "lft"
//! assert_eq!(ext.as_str(), "lft");
//! ```

use std::fmt;

/// Represents the file extension for raw inputs or pipeline artifacts.
///
/// # Variants
///
/// - `Pddl`: Planning Domain Definition Language raw input (`.pddl`)
/// - `Hddl`: Hierarchical Domain Definition Language raw input (`.hddl`)
/// - `Parsed`: Parsed pipeline artifact (`.prs`)
/// - `Lifted`: Lifted pipeline artifact (`.lft`)
/// - `Ground`: Grounded pipeline artifact (`.grd`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    /// Raw PDDL input
    Pddl,
    /// Raw HDDL input
    Hddl,
    /// Parsed intermediate file
    Parsed,
    /// Lifted intermediate file
    Lifted,
    /// Grounded intermediate file
    Grounded,
}

impl Extension {
    /// Returns the string representation of the file extension.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use your_crate::Extension;
    ///
    /// assert_eq!(Extension::Pddl.as_str(), "pddl");
    /// assert_eq!(Extension::Ground.as_str(), "grd");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Pddl => "pddl",
            Extension::Hddl => "hddl",
            Extension::Parsed => "prs",
            Extension::Lifted => "lft",
            Extension::Grounded => "grd",
        }
    }
}

impl fmt::Display for Extension {
    /// Formats the `Extension` as a human-readable string (its file extension).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use your_crate::Extension;
    ///
    /// let ext = Extension::Lifted;
    /// println!("{}", ext); // prints "lft"
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
