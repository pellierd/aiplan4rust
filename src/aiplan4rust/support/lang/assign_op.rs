//! Assignment operations used in syntax and mathematical modeling.
//!
//! This module defines the `AssignOp` enum representing various assignment
//! and value modification operations applicable to variables in syntax
//! problems or formal models, such as PDDL numeric effects.
//!
//! # Variants
//! - `Assign`: simple assignment operation.
//! - `ScaleUp`: multiply the value by a factor.
//! - `ScaleDown`: divide the value by a factor.
//! - `Increase`: add a value.
//! - `Decrease`: subtract a value.
//!
//! # Features
//!
//! - Provides user-friendly display strings for each operation via the
//!   `Display` trait implementation.
//! - Implements `InternerDisplay` and `SyntaxDisplay` traits for integration
//!   with the interning and syntax formatting systems.
//! - Delegates formatting to the standard `Display` trait as interner
//!   resolution is not required.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lang::AssignOp;
//!
//! let op = AssignOp::Increase;
//! assert_eq!(format!("{}", op), ":increase");
//! ```
//!
//! # Integration
//!
//! Works seamlessly with lexer tokens and syntax display for syntax languages.

use crate::aiplan4rust::support::interner::{InternerDisplay, SymbolInterner};
use crate::aiplan4rust::syntax::lexer::token::ASSIGN;
use crate::aiplan4rust::syntax::lexer::token::DECREASE;
use crate::aiplan4rust::syntax::lexer::token::INCREASE;
use crate::aiplan4rust::syntax::lexer::token::SCALE_DOWN;
use crate::aiplan4rust::syntax::lexer::token::SCALE_UP;
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// Represents assignment operations that can be used in syntax and mathematical models.
///
/// This enum defines various types of assignment or modification operations that can be applied
/// to variables or parameters in a syntax problem or formal model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssignOp {
    /// Basic assignment operation (sets a value).
    Assign,
    /// Scales up the value by a factor.
    ScaleUp,
    /// Scales down the value by a factor.
    ScaleDown,
    /// Increases the value by a certain amount.
    Increase,
    /// Decreases the value by a certain amount.
    Decrease,
}

/// Implements the `fmt::Display` trait for the `AssignOp` enum.
///
/// This implementation provides a user-friendly string representation
/// for each variant of `AssignOp`. It allows the enum to be formatted
/// as readable text when printed or logged, which is helpful for
/// debugging and displaying the operator in syntax syntax.
///
/// Each variant is mapped to a corresponding constant string:
/// - `AssignOp::Assign` -> `ASSIGN`
/// - `AssignOp::ScaleUp` -> `SCALE_UP`
/// - `AssignOp::ScaleDown` -> `SCALE_DOWN`
/// - `AssignOp::Increase` -> `INCREASE`
/// - `AssignOp::Decrease` -> `DECREASE`
///
/// # Example
///
/// ```
/// let op = AssignOp::Increase;
/// assert_eq!(format!("{}", op), ":increase");
/// ```
impl fmt::Display for AssignOp {
    /// Formats the `AssignOp` enum as its string representation.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssignOp::Assign => write!(f, "{}", ASSIGN),
            AssignOp::ScaleUp => write!(f, "{}", SCALE_UP),
            AssignOp::ScaleDown => write!(f, "{}", SCALE_DOWN),
            AssignOp::Increase => write!(f, "{}", INCREASE),
            AssignOp::Decrease => write!(f, "{}", DECREASE),
        }
    }
}

/// Implements the `DisplayWithInterner` trait for `AssignOp`.
///
/// Since `AssignOp` can be formatted directly using the standard
/// `Display` trait, this implementation simply delegates to it.
///
/// The `interner` parameter is unused because `AssignOp` does not
/// require any interner-based resolution.
///
/// # Example
///
/// ```
/// let op = AssignOp::Assign;
/// let s = format!("{}", op); // Using Display implementation
/// ```
impl InternerDisplay for AssignOp {
    /// Formats the `AssignOp` using the provided formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - The string interner (unused).
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating the result of the formatting operation.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &SymbolInterner) -> fmt::Result {
        // Delegate to the standard Display implementation.
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for `AssignOp`.
///
/// This implementation formats the `AssignOp` by delegating
/// to the standard `Display` trait, relying on the blanket
/// implementation of `DisplaySyntax` for types implementing
/// `DisplayWithInterner`.
///
/// The `_interner` parameter is unused.
///
/// # Example
///
/// ```
/// let op = AssignOp::Assign;
/// let s = op.to_string_with_interner(&interner);
/// assert_eq!(s, ":assign");
/// ```
impl SyntaxInternerDisplay for AssignOp {
    /// Formats the `AssignOp` using the given formatter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `_interner` - Unused string interner parameter.
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
