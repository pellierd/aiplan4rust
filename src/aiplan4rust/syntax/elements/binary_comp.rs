use crate::aiplan4rust::syntax::lexer::token::EQUAL;
use crate::aiplan4rust::syntax::lexer::token::GREATER;
use crate::aiplan4rust::syntax::lexer::token::GREATER_EQ;
use crate::aiplan4rust::syntax::lexer::token::LESS;
use crate::aiplan4rust::syntax::lexer::token::LESS_EQ;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

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

/// Implements the `DisplayWithInterner` trait for `BinaryComp`.
///
/// Since `BinaryComp` can be directly formatted via the standard
/// `Display` trait, this implementation simply delegates to it.
///
/// The `interner` parameter is unused because `BinaryComp` does not
/// require any interner-based resolution.
///
/// # Example
///
/// ```
/// let comp = BinaryComp::Eq;
/// let s = format!("{}", comp); // Using Display implementation
/// ```
impl DisplayWithInterner for BinaryComp {
    fn fmt_with(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        // Delegate to the standard Display implementation.
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `DisplaySyntax` trait for `BinaryComp`.
///
/// This implementation relies on the blanket implementation of
/// `DisplaySyntax` for all types that implement `DisplayWithInterner`,
/// so it is left empty.
///
/// # Example
///
/// ```
/// let comp = BinaryComp::Eq;
/// let s = comp.to_string_with_interner(&interner); // Uses DisplayWithInterner under the hood
/// ```
impl DisplaySyntax for BinaryComp {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
