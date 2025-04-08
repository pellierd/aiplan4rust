use crate::aiplan4rust::parser::lexer::token::{EQUAL, GREATER, GREATER_EQ, LESS, LESS_EQ};
use crate::aiplan4rust::pddl_display::PDDLDisplay;
use serde::{Deserialize, Serialize};
use std::fmt;

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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryComp {
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

impl fmt::Display for BinaryComp {
    /// Formats the binary comparison operator as its standard string representation.
    ///
    /// This implementation ensures that the operator is displayed using its conventional
    /// mathematical notation (`>`, `<`, `=`, `>=`, `<=`).
    ///
    /// # Parameters
    /// - `f`: The formatter used to output the formatted string.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of writing the formatted output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryComp::Greater => write!(f, "{}", GREATER),
            BinaryComp::Less => write!(f, "{}", LESS),
            BinaryComp::Equal => write!(f, "{}", EQUAL),
            BinaryComp::GreaterEq => write!(f, "{}", GREATER_EQ),
            BinaryComp::LessEq => write!(f, "{}", LESS_EQ),
        }
    }
}

impl PDDLDisplay for BinaryComp {}
