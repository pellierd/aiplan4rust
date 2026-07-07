use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use std::fmt;

/// Represents a Datalog rule in the form `Head :- Body.`.
///
/// A rule defines a logical implication where the `head` atom is derived as a
/// new fact if all atoms in the `body` (the conjunction of premises) are
/// satisfied by the current state of the database.
///
/// This structure is designed to be lightweight, while allowing the
/// [`DatalogEngine`](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::old_engine::DatalogEngine)
/// to perform high-level optimizations such as join reordering.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The conclusion or the fact produced by this rule.
    head: Atom,
    /// The list of atoms that must be satisfied to trigger the rule.
    body: Vec<Atom>,
}

impl Rule {
    /// Creates a new Datalog rule.
    ///
    /// # Arguments
    /// * `head` - The atom that will be inferred.
    /// * `body` - A collection of atoms representing the rule's conditions.
    #[inline]
    pub fn new(head: Atom, body: Vec<Atom>) -> Self {
        Self { head, body }
    }

    /// Returns a reference to the rule's head (conclusion).
    #[inline]
    pub fn head(&self) -> &Atom {
        &self.head
    }

    /// Returns a reference to the rule's body (conditions).
    #[inline]
    pub fn body(&self) -> &[Atom] {
        &self.body
    }

    /// Provides mutable access to the rule's body.
    ///
    /// This method is restricted to the crate level (`pub(crate)`). It is
    /// intended for the engine to perform join order optimizations (greedy
    /// reordering) without exposing mutable internals to the public API.
    #[inline]
    pub(crate) fn body_mut(&mut self) -> &mut Vec<Atom> {
        &mut self.body
    }
}

impl fmt::Display for Rule {
    /// Formats the rule using standard Datalog/Prolog syntax.
    ///
    /// Facts (rules with an empty body) are printed as just the head followed by a dot.
    /// Rules with conditions use the `:-` separator.
    ///
    /// # Example
    /// `sk_10(?v0) :- sk_1(?v0), sk_2(c42).`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the conclusion (Head)
        write!(f, "{}", self.head)?;

        // Display the conditions (Body) if they exist
        if !self.body.is_empty() {
            write!(f, " :- ")?;
            for (i, atom) in self.body.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", atom)?;
            }
        }

        // Every Datalog rule ends with a period
        write!(f, ".")
    }
}
