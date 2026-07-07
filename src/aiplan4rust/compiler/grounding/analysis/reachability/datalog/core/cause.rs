use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use std::fmt;

/// Explains the structural origin or trigger mechanism of an action effect during the grounding phase.
///
/// This enum categorizes how a specific effect was derived within the Datalog engine,
/// distinguishing between unconditional action effects and conditional effects mediated by a pivot.
///
/// # Memory Layout & Performance
///
/// This type implements [`Clone`] but **cannot** implement [`Copy`] because the [`Cause::Pivot`]
/// variant wraps an [`Atom`], which contains a heap-allocated `Vec`. Consequently, duplicating a
/// `Cause::Pivot` instance requires a deep copy of its underlying term vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Cause {
    /// The effect is unconditional and triggered systematically by the activation of the parent action.
    Action,
    /// The effect is conditional (e.g., inside a `When` block) and relies on an auxiliary conditional
    /// predicate or pivot atom as its derivation premise.
    Pivot(Atom),
}

impl fmt::Display for Cause {
    /// Formats the trigger cause for debugging, graph visualization, and logging output.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` upon successful formatting, or a [`fmt::Error`] if the underlying
    /// destination stream fails to accept characters.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action => write!(f, "Action"),
            Self::Pivot(atom) => write!(f, "Pivot({})", atom),
        }
    }
}
