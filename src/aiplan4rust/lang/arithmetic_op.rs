use crate::aiplan4rust::syntax::lexer::token::ADD;
use crate::aiplan4rust::syntax::lexer::token::DIV;
use crate::aiplan4rust::syntax::lexer::token::MUL;
use crate::aiplan4rust::syntax::lexer::token::SUB;
use crate::aiplan4rust::syntax::PlanningDisplay;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

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

impl fmt::Display for ArithmeticOp {
    /// Implements the `fmt::Display` trait for the `ArithmeticOp` enum.
    ///
    /// This method allows `ArithmeticOp` to be formatted as a string when printed
    /// using formatting macros such as `println!`. It provides a string representation
    /// of the arithmetic operation, making it easier to display and debug arithmetic expr.
    ///
    /// # Arguments
    /// - `f`: A mutable reference to the formatter, which is used to build the output string.
    ///
    /// # Returns
    /// - A `fmt::Result` indicating the success or failure of the formatting operation.
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
/// Since `ArithmeticOp` can be directly formatted via the standard
/// `Display` trait, this implementation simply delegates to it.
///
/// The `interner` parameter is unused because `ArithmeticOp` does not
/// require any interner-based resolution.
///
/// # Example
///
/// ```
/// let op = ArithmeticOp::Add;
/// let s = format!("{}", op); // Using standard Display implementation
/// ```
impl DisplayWithInterner for ArithmeticOp {
    fn fmt_with(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        // Delegate to the standard Display implementation.
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `DisplaySyntax` trait for `ArithmeticOp`.
///
/// This implementation relies on the blanket implementation of
/// `DisplaySyntax` for all types that implement `DisplayWithInterner`,
/// so it is left empty.
///
/// # Example
///
/// ```
/// let op = ArithmeticOp::Add;
/// // Uses DisplayWithInterner under the hood via DisplaySyntax
/// let s = op.to_string_with_interner(&interner);
/// ```
impl PlanningDisplay for ArithmeticOp {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
