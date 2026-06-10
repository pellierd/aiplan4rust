use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use std::fmt;

/// Represents a logical atom in a Datalog rule.
///
/// An `Atom` consists of a predicate (identified by its [`AtomSkeletonId`]) and
/// a sequence of [`Term`]s (variables or constants). In the context of
/// reachability analysis, atoms appear in the head or body of rules to define
/// how facts are derived.
///
/// Atoms are the "schema" level objects that the engine uses to query or
/// update the [`Database`](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::database::Database).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Atom {
    /// The unique identifier for the predicate signature (Name + Parameter Types).
    /// This ID serves as the key to locate the corresponding Relation in the Database.
    skeleton_id: AtomSkeletonId,
    /// The actual arguments for this instance, which can be [`Term::Variable`]
    /// or [`Term::Constant`].
    terms: Vec<Term>,
}

impl Atom {
    // Zone des Built-ins décalée pour ne pas mordre sur le bit de signe (MSB)
    // 1 << 62 est une valeur immense, mais le bit 63 reste à 0.
    pub const BUILTIN_ZONE_START: usize = 1 << 62;

    /// ID immuable pour l'égalité
    pub const EQUALITY_ID: usize = Self::BUILTIN_ZONE_START;

    /// Creates a new `Atom` with the given skeleton ID and terms.
    ///
    /// By debug, the atom is not negated.
    ///
    /// # Arguments
    /// * `skeleton_id` - The identifier mapping to a specific predicate in the domain.
    /// * `terms` - The list of variables or constants associated with this predicate.
    pub fn new(skeleton_id: AtomSkeletonId, terms: Vec<Term>) -> Self {
        Self { skeleton_id, terms }
    }

    /// Creates a new equality atom representing `(= t1 t2)`.
    ///
    /// This uses the reserved [`EQUALITY_ID`] located in the built-in zone
    /// of the predicate space. Equality atoms are handled specifically by
    /// the Datalog engine during the grounding and unification process.
    pub fn equality(t1: Term, t2: Term) -> Self {
        Self {
            skeleton_id: AtomSkeletonId::from(Self::EQUALITY_ID),
            terms: vec![t1, t2],
        }
    }

    /// Toggles the negation state of the atom.
    ///
    /// This method flips the Most Significant Bit (MSB) of the underlying
    /// [`AtomSkeletonId`]. If the atom was positive, it becomes negated
    /// (representing a Delete Effect or a negative precondition), and vice versa.
    pub fn negated(&mut self) {
        let new_state = !self.is_negated();
        self.skeleton_id.set_negated(new_state);
    }

    /// Vérifie si cet atome est une égalité (ou une inégalité).
    /// L'ID est "nettoyé" par .as_usize() avant la comparaison.
    #[inline(always)]
    pub fn is_equality(&self) -> bool {
        self.skeleton_id.as_usize() == Self::EQUALITY_ID
    }

    /// Retourne `true` si l'atome est négatif en interrogeant le bit MSB de l'ID.
    #[inline(always)]
    pub fn is_negated(&self) -> bool {
        self.skeleton_id.is_negated()
    }

    /// Permet de forcer un état de négation spécifique.
    #[inline(always)]
    pub fn set_negated(&mut self, negated: bool) {
        self.skeleton_id.set_negated(negated);
    }

    /// Returns the unique identifier of the atom's skeleton.
    #[inline]
    pub fn skeleton_id(&self) -> AtomSkeletonId {
        self.skeleton_id
    }

    /// Updates the skeleton identifier (predicate ID).
    ///
    /// This is primarily used during the encoding phase to transform a base
    /// predicate ID into a shifted ID (e.g., applying the `negation_offset`
    /// to represent a delete effect in the Datalog engine).
    #[inline]
    pub fn set_skeleton_id(&mut self, new_id: AtomSkeletonId) {
        self.skeleton_id = new_id;
    }

    /// Returns a slice containing the terms of this atom.
    #[inline]
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Replaces the entire sequence of terms in the atom.
    ///
    /// While `terms_mut` is preferred for in-place updates (like aliasing),
    /// this method allows for a full structural swap of the atom's arguments.
    #[inline]
    pub fn set_terms(&mut self, new_terms: Vec<Term>) {
        self.terms = new_terms;
    }

    /// Returns a mutable slice of the atom's terms.
    ///
    /// This is primarily used during the encoding phase for variable aliasing,
    /// logic, or grounding, allowing in-place modification of terms
    /// without reallocating the underlying vector.
    #[inline]
    pub fn terms_mut(&mut self) -> &mut [Term] {
        &mut self.terms
    }

    /// Returns the arity (the number of terms) of the atom.
    #[inline]
    pub fn arity(&self) -> usize {
        self.terms.len()
    }
}

impl fmt::Display for Atom {
    /// Formats the atom for debug output and logging.
    ///
    /// Utilise is_negated() qui lit le bit MSB de l'ID.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_neg = self.is_negated();

        if is_neg {
            write!(f, "not(")?;
        }

        // On utilise .as_usize() pour ne pas afficher le bit de signe
        // dans le nom du prédicat (évite d'avoir not(not_AS#...))
        write!(f, "sk_{}(", self.skeleton_id.as_usize())?;

        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", term)?;
        }
        write!(f, ")")?;

        if is_neg {
            write!(f, ")")?;
        }
        Ok(())
    }
}
