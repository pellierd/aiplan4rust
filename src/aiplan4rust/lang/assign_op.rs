use crate::aiplan4rust::syntax::lexer::token::ASSIGN;
use crate::aiplan4rust::syntax::lexer::token::DECREASE;
use crate::aiplan4rust::syntax::lexer::token::INCREASE;
use crate::aiplan4rust::syntax::lexer::token::SCALE_DOWN;
use crate::aiplan4rust::syntax::lexer::token::SCALE_UP;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// Represents assignment operations that can be used in planning and mathematical models.
///
/// This enum defines various types of assignment or modification operations that can be applied
/// to variables or parameters in a planning problem or formal model.
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

impl fmt::Display for AssignOp {
    /// Formats the `AssignOp` enum as a string representation for display purposes.
    ///
    /// This implementation of the `fmt::Display` trait enables the `AssignOp` enum to be
    /// formatted into a user-friendly string representation for displaying to the user.
    /// Each variant of the `AssignOp` enum is mapped to a corresponding string to provide
    /// a clear and readable output when the enum is printed or logged.
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
/// Since `AssignOp` can be directly formatted via the standard
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
impl DisplayWithInterner for AssignOp {
    fn fmt_with(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        // Delegate to the standard Display implementation.
        fmt::Display::fmt(self, f)
    }
}

/// Implements the `DisplaySyntax` trait for `AssignOp`.
///
/// This implementation relies on the blanket implementation of
/// `DisplaySyntax` for all types that implement `DisplayWithInterner`,
/// so it is left empty.
///
/// # Example
///
/// ```
/// let op = AssignOp::Assign;
/// let s = op.to_string_with_interner(&interner); // Uses DisplayWithInterner under the hood
/// ```
impl DisplaySyntax for AssignOp {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
