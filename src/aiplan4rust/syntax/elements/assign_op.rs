use crate::aiplan4rust::pddl_display::PDDLDisplay;
use crate::aiplan4rust::syntax::token::{ASSIGN, DECREASE, INCREASE, SCALE_DOWN, SCALE_UP};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents assignment operations that can be used in planning and mathematical models.
///
/// This enum defines various types of assignment or modification operations that can be applied
/// to variables or parameters in a planning problem or formal model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl PDDLDisplay for AssignOp {}
