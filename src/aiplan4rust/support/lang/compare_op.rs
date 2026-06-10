//! Binary comparison operators used in conditions and constraints.
//!
//! This module defines the `BinaryComp` enum, representing the standard
//! comparison operators used in formal models such as PDDL, for expressing
//! numeric and logical constraints.
//!
//! # Variants
//! - `Greater` (`>`): Greater than comparison.
//! - `Less` (`<`): Less than comparison.
//! - `Equal` (`=`): Equality comparison.
//! - `GreaterEq` (`>=`): Greater than or equal to comparison.
//! - `LessEq` (`<=`): Less than or equal to comparison.
//!
//! # Features
//!
//! - Implements `Display` to format each variant as its symbolic operator string.
//! - Implements `InternerDisplay` and `SyntaxDisplay` traits for integration with
//!   interning and syntax display systems.
//! - Formatting delegates to the standard `Display` implementation as interner
//!   resolution is not required.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lang::BinaryComp;
//!
//! let op = BinaryComp::GreaterEq;
//! assert_eq!(op.to_string(), ">=");
//! ```

use crate::aiplan4rust::support::interner::{InternerDisplay, SymbolInterner};
use crate::aiplan4rust::syntax::lexer::token::EQUAL;
use crate::aiplan4rust::syntax::lexer::token::GREATER;
use crate::aiplan4rust::syntax::lexer::token::GREATER_EQ;
use crate::aiplan4rust::syntax::lexer::token::LESS;
use crate::aiplan4rust::syntax::lexer::token::LESS_EQ;
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

///
/// This enumeration defines the common comparison operators that can be used in
/// conditions and constraints, particularly in the context of PDDL or other formal models.
///
/// # Variants
/// - `Greater` (`>`): Represents a "greater than" comparison.
/// - `Less` (`<`): Represents a "less than" comparison.
/// - `Equal` (`=`): Represents an "equal to" comparison.
/// - `GreaterEq` (`>=`): Represents a "greater than or equal to" comparison.
/// - `LessEq` (`<=`): Represents a "less than or equal to" comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompareOp {
    /// Represents a "greater than" comparison (`>`).
    Greater,
    /// Represents a "less than" comparison (`<`).
    Less,
    /// Represents an "equal to" comparison (`=`).
    Equal,
    /// Represents a "greater than or equal to" comparison (`>=`).
    GreaterEq,
    /// Represents a "less than or equal to" comparison (`<=`).
    LessEq,
}

/// Implements the `Display` trait for `BinaryComp`.
///
/// This allows `BinaryComp` variants to be formatted as their
/// conventional mathematical operator strings (`>`, `<`, `=`, `>=`, `<=`).
///
/// # Example
///
/// ```
/// let comp = BinaryComp::GreaterEq;
/// assert_eq!(comp.to_string(), ">=");
/// ```
impl fmt::Display for CompareOp {
    /// Formats the `BinaryComp` as a string representing the operator.
    ///
    /// Converts the variant into its corresponding symbol:
    /// - `Greater` => `">"`
    /// - `Less` => `"<"`
    /// - `Equal` => `"="`
    /// - `GreaterEq` => `">="`
    /// - `LessEq` => `"<="`
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output string.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure of the formatting operation.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompareOp::Greater => write!(f, "{}", GREATER),
            CompareOp::Less => write!(f, "{}", LESS),
            CompareOp::Equal => write!(f, "{}", EQUAL),
            CompareOp::GreaterEq => write!(f, "{}", GREATER_EQ),
            CompareOp::LessEq => write!(f, "{}", LESS_EQ),
        }
    }
}

/// Implements the `DisplayWithInterner` trait for `BinaryComp`.
///
/// Since `BinaryComp` can be formatted directly using the standard
/// `Display` trait, this implementation simply delegates to it.
///
/// The `interner` parameter is unused because `BinaryComp` does not
/// require interner-based resolution.
///
/// # Example
///
/// ```
/// let comp = BinaryComp::Eq;
/// let s = format!("{}", comp); // Uses the Display implementation
/// ```
impl InternerDisplay for CompareOp {
    /// Formats the `BinaryComp` using the given formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - The interner, unused in this implementation.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &SymbolInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for `BinaryComp`.
///
/// This implementation formats `BinaryComp` by delegating to its standard
/// `Display` implementation, as no interner-based resolution is needed.
///
/// The `_interner` parameter is unused.
///
/// # Example
///
/// ```
/// let comp = BinaryComp::Eq;
/// let s = comp.to_string_with_interner(&interner); // Delegates to Display
/// ```
impl SyntaxInternerDisplay for CompareOp {
    /// Formats the `BinaryComp` using the provided formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - The string interner, unused here.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        _interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        fmt::Display::fmt(self, f)
    }
}
