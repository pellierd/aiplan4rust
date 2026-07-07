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
/// [Rules](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule)
/// and [Atoms](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom).
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
    /// Formats the term using standard Datalog syntactic structures for readability.
    ///
    /// This method delegates formatting directly to the underlying identifier types,
    /// prefixing variables with a standard `?` marker.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` if formatting completes successfully, or a [`fmt::Error`] upon stream buffer failure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    /// # use crate::aiplan4rust::support::lang::{VariableId, ObjectId};
    /// let var = Term::Variable(VariableId::from(0));
    /// let con = Term::Constant(ObjectId::from(42));
    ///
    /// assert_eq!(format!("{}", var), "?v#0");
    /// assert_eq!(format!("{}", con), "o#42");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Prepends the standard Datalog variable symbol '?' to the VariableId display representation
            Term::Variable(id) => write!(f, "?{}", id),
            // Directly forwards the formatting token to the ObjectId display implementation
            Term::Constant(id) => write!(f, "{}", id),
        }
    }
}
