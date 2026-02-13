//! Arithmetic operations used in PDDL and HDDL expr and logical comparisons.
//!
//! This module defines the `ArithmeticOp` enum representing basic arithmetic
//! operators (addition, subtraction, multiplication, division) commonly used
//! in numeric expr within PDDL (Planning Domain Definition Language).
//!
//! # Features
//!
//! - Enumerates the four fundamental arithmetic operators.
//! - Implements `Display` to format operators as their symbolic representation (`+`, `-`, `*`, `/`).
//! - Implements `InternerDisplay` and `SyntaxDisplay` traits to support
//!   formatting within the syntax and interning framework, delegating to the
//!   standard `Display` implementation since no interning is required.
//!
//! # Usage Example
//!
//! ```rust
//! use your_crate::ArithmeticOp;
//!
//! let op = ArithmeticOp::Add;
//! assert_eq!(format!("{}", op), "+");
//! ```
//!
//! # Integration
//!
//! This module integrates with the lexer tokens for arithmetic operators
//! and the syntax display system, allowing seamless formatting of PDDL numeric
//! expr in user-friendly syntax.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::lexer::token::ADD;
use crate::aiplan4rust::syntax::lexer::token::DIV;
use crate::aiplan4rust::syntax::lexer::token::MUL;
use crate::aiplan4rust::syntax::lexer::token::SUB;
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// Represents binary comparison operators used in logical and
/// Represents arithmetic operations that can be used in PDDL expr.
///
/// This enumeration defines the basic arithmetic operators commonly found
/// in PDDL (Planning Domain Definition Language), which are used in numeric
/// expr for modifying and evaluating numerical state variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArithmeticOp {
    /// Represents the subtraction operation (`-`).
    Sub,
    /// Represents the division operation (`/`).
    Div,
    /// Represents the addition operation (`+`).
    Add,
    /// Represents the multiplication operation (`*`).
    Mul,
}

/// Implements the `fmt::Display` trait for the `ArithmeticOp` enum.
///
/// This implementation enables `ArithmeticOp` values to be formatted as strings
/// using Rust’s formatting macros (e.g., `println!`, `format!`). It provides
/// a human-readable representation of the arithmetic operation symbols, which
/// is useful for displaying and debugging arithmetic expr.
///
/// # Example
///
/// ```
/// let op = ArithmeticOp::Add;
/// println!("{}", op); // prints "+"
/// ```
impl fmt::Display for ArithmeticOp {
    /// Formats the `ArithmeticOp` as its corresponding symbol.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to the formatter used to build the output string.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether the formatting succeeded or failed.
    ///
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArithmeticOp::Sub => write!(f, "{}", SUB),
            ArithmeticOp::Div => write!(f, "{}", DIV),
            ArithmeticOp::Add => write!(f, "{}", ADD),
            ArithmeticOp::Mul => write!(f, "{}", MUL),
        }
    }
}

/// Implements the `DisplayWithInterner` trait for `ArithmeticOp`.
///
/// This implementation formats the `ArithmeticOp` by delegating
/// to the standard `Display` trait, as no interner-based resolution
/// is needed.
///
/// The `interner` parameter is unused.
///
/// # Example
///
/// ```
/// let op = ArithmeticOp::Add;
/// let s = format!("{}", op); // Uses the standard Display trait.
/// ```
impl InternerDisplay for ArithmeticOp {
    /// Formats the `ArithmeticOp` using the given formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - The string interner (unused).
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for `ArithmeticOp`.
///
/// This implementation provides formatting for `ArithmeticOp` by
/// delegating to the standard `Display` trait. It leverages the
/// blanket implementation of `DisplaySyntax` for types implementing
/// `DisplayWithInterner`.
///
/// Since `ArithmeticOp` does not require interner-based resolution,
/// the `_interner` parameter is unused.
///
/// # Example
///
/// ```
/// let op = ArithmeticOp::Add;
/// let s = op.fmt_planning(&mut formatter, &interner)?;
/// // Output uses the standard Display implementation.
/// ```
impl SyntaxInternerDisplay for ArithmeticOp {
    /// Formats the `ArithmeticOp` using the given formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    /// * `_interner` - The string interner, unused for this type_checker.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting succeeded or failed.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        _interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        fmt::Display::fmt(self, f)
    }
}
