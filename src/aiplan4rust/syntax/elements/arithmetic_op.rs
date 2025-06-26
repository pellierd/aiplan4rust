use crate::aiplan4rust::syntax::lexer::token::ADD;
use crate::aiplan4rust::syntax::lexer::token::DIV;
use crate::aiplan4rust::syntax::lexer::token::MUL;
use crate::aiplan4rust::syntax::lexer::token::SUB;
use crate::aiplan4rust::syntax::SyntaxDisplay;

use serde::Deserialize;
use serde::Serialize;

use std::fmt;

/// Represents binary comparison operators used in logical and
/// Represents arithmetic operations that can be used in PDDL expression.
///
/// This enumeration defines the basic arithmetic operators commonly found
/// in PDDL (Planning Domain Definition Language), which are used in numeric
/// expression for modifying and evaluating numerical state variables.
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
    /// of the arithmetic operation, making it easier to display and debug arithmetic expression.
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

impl SyntaxDisplay for ArithmeticOp {}
