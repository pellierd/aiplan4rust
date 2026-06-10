use crate::aiplan4rust::support::lang::{ObjectId, VariableId};
use std::fmt;

/// Represents a term within a Datalog atom.
///
/// In the context of reachability analysis, a term is either a placeholder
/// ([`Variable`][Term::Variable]) that will be bound during unification,
/// or a fixed entity ([`Constant`][Term::Constant]) representing a specific
/// object in the planning problem.
///
/// This enum is fundamental for defining the structure of both
/// [Rules](crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule)
/// and [Atoms](crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Term {
    /// A logical variable used in rules to represent an unspecified object.
    ///
    /// During the evaluation of a rule, variables are substituted with
    /// concrete [`ObjectId`]s from the database.
    Variable(VariableId),
    /// A concrete object identifier from the PDDL problem.
    ///
    /// Constants are fixed and do not change during the saturation process.
    Constant(ObjectId),
}

impl fmt::Display for Term {
    /// Formats the term using standard PDDL-style conventions for readability.
    ///
    /// # Output Format
    /// - **Variables**: Prefixed with `?v` followed by the unique ID (e.g., `?v0`, `?v1`).
    /// - **Constants**: Prefixed with `c` followed by the unique ID (e.g., `c101`, `c42`).
    ///
    /// # Example
    /// ```
    /// # use aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
    /// # use aiplan4rust::lang::{VariableId, ObjectId};
    /// let var = Term::Variable(VariableId::from(0));
    /// let con = Term::Constant(ObjectId::from(42));
    ///
    /// assert_eq!(format!("{}", var), "?v0");
    /// assert_eq!(format!("{}", con), "c42");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // On utilise .as_usize() pour ne pas déclencher le "v#" de VariableId
            Term::Variable(id) => write!(f, "?{}", id),

            // On utilise .as_usize() pour ne pas déclencher le "o#" de ObjectId (ou "co")
            Term::Constant(id) => write!(f, "{}", id),
        }
    }
}
