use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use std::fmt;

/// Represents a logical atom within a Datalog rule or fact representation.
///
/// An `Atom` is the foundational relational component of the Datalog grounding engine.
/// It binds a specific predicate relation (identified by its [`AtomSkeletonId`]) to an ordered
/// sequence of arguments ([`Term`]s), which may encompass either variables or constants.
/// In the context of reachability analysis, atoms populate the heads and bodies of Horn clauses
/// to dictate how newly derived facts propagate through the database network.
///
/// Atoms function as schema-level templates that the engine evaluates, unifies, and translates
/// into concrete relational tuples when querying or updating the
/// [`Database`](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database).
///
/// # Memory Layout & Performance
///
/// * **Packed Fields**: The [`AtomSkeletonId`] packs boolean control flags (such as the negation status
///   and built-in zone markers) directly into the unused upper bits of the predicate index. This keeps
///   the footprint lightweight and optimizes CPU cache locality during intensive grounding loops.
/// * **Heap Allocation**: The dynamic vector of terms allows for arbitrary predicate arities, though
///   in practice, most Datalog rules rely on low-arity schemas (typically $\le 4$) to prevent
///   exponential combinatorics during join operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Atom {
    /// The unique identifier for the predicate signature (encompassing name, arity, and parameter types).
    /// This packed ID acts as the index key to locate and modify the matching relation block inside the database.
    skeleton_id: AtomSkeletonId,
    /// The contiguous sequence of arguments assigned to this instance, matching either
    /// [`Term::Variable`] placeholders or evaluated [`Term::Constant`] literals.
    terms: Vec<Term>,
}

impl Atom {
    /// Offset defining the start of the built-in zone.
    /// Shifted to avoid interfering with the negation flag (bit 60) or sign bits.
    pub const BUILTIN_ZONE_START: usize = 1 << 62;

    /// Immutable identifier reserved for structural equality predicates.
    pub const EQUALITY_ID: usize = Self::BUILTIN_ZONE_START;

    /// Creates a new `Atom` with the given skeleton identifier and terms.
    ///
    /// By default, a newly constructed atom is initialized as a positive (non-negated) literal.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The unique [`AtomSkeletonId`] mapping to a specific predicate schema in the Datalog domain.
    /// * `terms` - A vector containing the variables or constants assigned as arguments to this predicate.
    ///
    /// # Return Value
    ///
    /// Returns a fresh [`Self`] instance wrapping the provided predicate ID and terms.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ auxiliary complexity, though moving the `Vec<Term>` may scale
    /// with the allocation overhead of the input terms.
    pub fn new(skeleton_id: AtomSkeletonId, terms: Vec<Term>) -> Self {
        Self { skeleton_id, terms }
    }

    /// Creates a new built-in equality atom representing the constraint `(= t1 t2)`.
    ///
    /// This method leverages the reserved [`Self::EQUALITY_ID`] located in the high-range
    /// built-in memory zone of the predicate space. Equality atoms are handled via
    /// dedicated optimized code paths in the Datalog engine during grounding and unification.
    ///
    /// # Arguments
    ///
    /// * `t1` - The first [`Term`] argument (left-hand side of the equality).
    /// * `t2` - The second [`Term`] argument (right-hand side of the equality).
    ///
    /// # Return Value
    ///
    /// Returns a binary [`Self`] instance configured with the reserved equality identifier.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ since it allocates a fixed contiguous buffer of exactly two terms.
    pub fn equality(t1: Term, t2: Term) -> Self {
        Self {
            skeleton_id: AtomSkeletonId::from(Self::EQUALITY_ID),
            terms: vec![t1, t2],
        }
    }

    /// Toggles the negation state of the atom in place.
    ///
    /// This method flips the negation control flag (typically bit 60) stored within the
    /// underlying [`AtomSkeletonId`]. If the atom was positive, it becomes negated
    /// (representing a negative literal or a delete effect), and vice versa.
    ///
    /// # Implementation Details
    ///
    /// The state inversion relies on reading the current bit flag via [`Self::is_negated()`],
    /// applying a logical NOT, and committing the updated state back to the bitfield.
    /// This prevents altering the base index or the built-in zone tracking bits.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ memory and execution footprint.
    pub fn negated(&mut self) {
        let new_state = !self.is_negated();
        self.skeleton_id.set_negated(new_state);
    }

    /// Checks whether this atom represents an equality (or inequality) constraint.
    ///
    /// This method inspects the raw underlying identifier (`as_raw_usize`) rather than
    /// the masked index to capture the built-in zone marker. It applies a bitwise
    /// mask to ignore the negation flag (bit 60), ensuring that both positive equalities
    /// and negated equalities (inequalities) are correctly identified.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the atom is structurally an equality relation (e.g., `x = y` or `x ≠ y`),
    /// and `false` otherwise.
    ///
    /// # Implementation Details
    ///
    /// The verification relies on comparing the predicate's skeleton identifier against the
    /// reserved [`Self::EQUALITY_ID`]. Because a negated equality (an inequality) flips the
    /// sign bit (bit 60), a direct raw comparison would fail. To prevent this, a bitwise NOT
    /// combined with an AND mask (`& !(1 << 60)`) clears the negation flag before performing
    /// the equality check.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ and memory $O(1)$, as it only performs a stack-based bitwise
    /// operation and an integer comparison.
    pub fn is_equality(&self) -> bool {
        // Mask out the negation bit (bit 60) so that inequalities are also recognized.
        (self.skeleton_id.as_raw_usize() & !(1 << 60)) == Self::EQUALITY_ID
    }

    /// Checks whether this atom is negated by evaluating the Most Significant Bit (MSB) of its identifier.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the atom carries the negation flag (bit 60 set to 1), representing a negative literal
    /// (e.g., `not(p(...))`), and `false` if it represents a positive literal.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ as it performs a direct bitwise operation delegated to the underlying [`AtomSkeletonId`].
    #[inline(always)]
    pub fn is_negated(&self) -> bool {
        self.skeleton_id.is_negated()
    }

    /// Explicitly forces or clears the negation state of the atom by modifying its internal identifier flags.
    ///
    /// # Arguments
    ///
    /// * `negated` - A boolean flag where `true` marks the atom as negated and `false` restores it to positive.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ since it mutates the control bits of the underlying integer index in place.
    #[inline(always)]
    pub fn set_negated(&mut self, negated: bool) {
        self.skeleton_id.set_negated(negated);
    }

    /// Returns the unique identifier of the atom's skeleton.
    ///
    /// # Return Value
    ///
    /// Returns a copy of the [`AtomSkeletonId`] containing the underlying predicate index and its packed layout flags.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ as [`AtomSkeletonId`] is a lightweight `Copy` type.
    #[inline]
    pub fn skeleton_id(&self) -> AtomSkeletonId {
        self.skeleton_id
    }

    /// Updates the skeleton identifier (predicate ID).
    ///
    /// This is primarily used during the encoding phase to transform a base
    /// predicate ID into a shifted ID (e.g., applying the `negation_offset`
    /// to represent a delete effect in the Datalog engine).
    ///
    /// # Arguments
    ///
    /// * `new_id` - The new [`AtomSkeletonId`] to associate with this atom.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$.
    #[inline]
    pub fn set_skeleton_id(&mut self, new_id: AtomSkeletonId) {
        self.skeleton_id = new_id;
    }

    /// Returns an immutable borrow of the terms sequence assigned to this atom.
    ///
    /// # Return Value
    ///
    /// Returns a shared slice reference (`&[Term]`) representing the ordered arguments of the predicate.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ to produce the slice reference.
    #[inline]
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Replaces the entire sequence of terms in the atom.
    ///
    /// While [`Self::terms_mut()`] is preferred for in-place updates (such as variable unification
    /// or aliasing), this method allows for a complete structural swap of the atom's arguments.
    ///
    /// # Arguments
    ///
    /// * `new_terms` - A vector containing the complete sequence of new [`Term`] arguments.
    ///
    /// # Complexity
    ///
    /// * **Time Complexity**: $O(1)$ auxiliary swap time if the vector allocation is moved, but triggers
    ///   the drop of the old vector, which takes $O(N)$ where $N$ is the previous arity.
    /// * **Memory Complexity**: Reallocates the internal buffer to hold the new sequence length.
    #[inline]
    pub fn set_terms(&mut self, new_terms: Vec<Term>) {
        self.terms = new_terms;
    }

    /// Returns a mutable slice of the atom's terms.
    ///
    /// This is primarily used during the encoding phase for variable aliasing,
    /// unification logic, or grounding, allowing fast in-place modifications of terms
    /// without reallocating the underlying vector wrapper.
    ///
    /// # Return Value
    ///
    /// Returns an exclusive mutable slice reference (`&mut [Term]`) over the atom's arguments.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ to slice into the contiguous vector buffer.
    #[inline]
    pub fn terms_mut(&mut self) -> &mut [Term] {
        &mut self.terms
    }

    /// Returns the arity (the number of terms/arguments) of the atom.
    ///
    /// # Return Value
    ///
    /// Returns a `usize` indicating the size of the underlying terms container.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ since vector length tracking is managed on the stack.
    #[inline]
    pub fn arity(&self) -> usize {
        self.terms.len()
    }
}

impl fmt::Display for Atom {
    /// Formats the atom for debug output, logging, and human-readable representation.
    ///
    /// This method converts the Datalog atom into a standard string representation.
    /// If the atom is negated, it wraps the predicate inside a `not(...)` block.
    /// To ensure the printed predicate identifier remains clean and readable, it utilizes
    /// `.as_usize()` to strip any internal bitwise control flags (such as negation or sign bits)
    /// from the raw ID.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` if the formatting operation succeeds, or a [`fmt::Error`]
    /// if the underlying writer fails to output the characters.
    ///
    /// # Implementation Details
    ///
    /// The formatting pipeline executes the following structural rendering steps:
    /// 1. Evaluates [`Self::is_negated()`] to prepend `not(` if necessary.
    /// 2. Prints the sanitized predicate prefix as `sk_{ID}(` using the masked index value.
    /// 3. Iterates over the internal [`Term`] slice, formatting each argument sequentially
    ///    with a comma-separated delimiter.
    /// 4. Appends the closing parentheses to balance the atom signature and the negation wrapper.
    ///
    /// # Complexity
    ///
    /// * **Time Complexity**: $O(N)$ where $N$ is the arity (number of terms) of the atom,
    ///   as it must visit and format each individual term argument.
    /// * **Memory Complexity**: $O(1)$ auxiliary space, since it streams the formatted characters
    ///   directly into the provided formatter buffer without allocating dynamic string buffers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_neg = self.is_negated();

        if is_neg {
            write!(f, "not(")?;
        }

        // Use `.as_usize()` to safely mask out control bits (e.g., negation flag),
        // preventing corrupted identifiers like `not(not_AS#...)` in the output logs.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, VariableId};

    /// Helper function to create a basic set of terms for testing.
    /// Uses a concrete enum variant (`Term::Variable`) to bypass the lack of `Default`.
    fn setup_dummy_terms() -> Vec<Term> {
        vec![Term::Variable(VariableId::from(0))]
    }

    /// **Objective**: Verify that a new positive atom is correctly initialized with its skeleton ID and terms.
    ///
    /// **Input**: `skeleton_id = 42`, `terms = [Term::Variable(0)]`
    ///
    /// **Expected Output**: A positive (non-negated) Atom with an arity of 1 and skeleton ID 42.
    #[test]
    fn test_new_atom_creation() {
        let sk_id = AtomSkeletonId::from(42);
        let terms = setup_dummy_terms();
        let atom = Atom::new(sk_id, terms.clone());

        assert_eq!(atom.skeleton_id().as_usize(), 42);
        assert_eq!(atom.arity(), terms.len());
        assert!(!atom.is_negated());
        assert!(!atom.is_equality());
    }

    /// **Objective**: Ensure that the built-in equality atom is created with the reserved `EQUALITY_ID` and holds exactly 2 terms.
    ///
    /// **Input**: `t1 = Term::Variable(0)`, `t2 = Term::Variable(1)`
    ///
    /// **Expected Output**: An atom identified as an equality predicate, located in the safe built-in zone (via raw representation), with arity 2.
    #[test]
    fn test_equality_atom_initialization() {
        let t1 = Term::Variable(VariableId::from(0));
        let t2 = Term::Variable(VariableId::from(1));
        let atom = Atom::equality(t1, t2);

        assert!(atom.is_equality());
        assert_eq!(atom.skeleton_id().as_raw_usize(), Atom::EQUALITY_ID);
        assert_eq!(atom.arity(), 2);
        assert!(!atom.is_negated());
    }

    /// **Objective**: Validate that toggling negation modifies the MSB/flag bit 60 without altering the underlying base predicate ID.
    ///
    /// **Input**: A fresh positive atom with skeleton ID 10.
    ///
    /// **Expected Output**: Alternates between negated (true) and positive (false). `as_usize()` must always return 10.
    #[test]
    fn test_negation_toggling_via_msb() {
        let sk_id = AtomSkeletonId::from(10);
        let mut atom = Atom::new(sk_id, setup_dummy_terms());

        assert!(!atom.is_negated());

        // First toggle: Positive -> Negated (Bit 60 turns to 1)
        atom.negated();
        assert!(atom.is_negated());
        assert_eq!(atom.skeleton_id().as_usize(), 10);

        // Second toggle: Negated -> Positive (Bit 60 turns to 0)
        atom.negated();
        assert!(!atom.is_negated());
        assert_eq!(atom.skeleton_id().as_usize(), 10);
    }

    /// **Objective**: Verify that forcing a specific negation state via `set_negated` behaves deterministically.
    ///
    /// **Input**: A fresh positive atom with skeleton ID 15.
    ///
    /// **Expected Output**: Reflects the exact boolean state assigned, regardless of the previous state.
    #[test]
    fn test_explicit_set_negated() {
        let sk_id = AtomSkeletonId::from(15);
        let mut atom = Atom::new(sk_id, setup_dummy_terms());

        atom.set_negated(true);
        assert!(atom.is_negated());

        atom.set_negated(true); // Redundant assignment
        assert!(atom.is_negated());

        atom.set_negated(false);
        assert!(!atom.is_negated());
    }

    /// **Objective**: Ensure that replacing the predicate skeleton ID works correctly (crucial for encoding offsets).
    ///
    /// **Input**: An atom initialized with ID 5, mutated to ID 100.
    ///
    /// **Expected Output**: The atom's skeleton ID changes to 100.
    #[test]
    fn test_set_skeleton_id_mutation() {
        let mut atom = Atom::new(AtomSkeletonId::from(5), setup_dummy_terms());
        atom.set_skeleton_id(AtomSkeletonId::from(100));

        assert_eq!(atom.skeleton_id().as_usize(), 100);
    }

    /// **Objective**: Check that replacing the entire term sequence updates the atom's internal vector and arity.
    ///
    /// **Input**: An atom with 1 term, updated with a new vector of 2 terms.
    ///
    /// **Expected Output**: Internal terms are replaced, and arity updates from 1 to 2.
    #[test]
    fn test_terms_sequence_replacement() {
        let mut atom = Atom::new(AtomSkeletonId::from(1), setup_dummy_terms());
        assert_eq!(atom.arity(), 1);

        let new_sequence = vec![
            Term::Variable(VariableId::from(0)),
            Term::Variable(VariableId::from(1)),
        ];
        atom.set_terms(new_sequence);
        assert_eq!(atom.arity(), 2);
    }

    /// **Objective**: Verify that an equality atom remains identified as an equality even when negated (representing an inequality).
    ///
    /// **Input**: An equality atom mutated via `.negated()`.
    ///
    /// **Expected Output**: `is_equality()` remains true, `is_negated()` becomes true, and arity stays 2.
    #[test]
    fn test_inequality_via_negated_equality() {
        let t1 = Term::Variable(VariableId::from(0));
        let t2 = Term::Variable(VariableId::from(1));
        let mut atom = Atom::equality(t1, t2);

        atom.negated();
        assert!(atom.is_negated());
        assert!(
            atom.is_equality(),
            "L'inégalité doit toujours être reconnue comme une égalité structurelle !"
        );
        assert_eq!(atom.arity(), 2);
    }

    /// **Objective**: Ensure that `terms_mut` allows in-place modification of terms without structural reallocation.
    ///
    /// **Input**: An atom with `Term::Variable(0)`, mutated to `Term::Variable(99)`.
    ///
    /// **Expected Output**: The term at index 0 is successfully updated to 99.
    #[test]
    fn test_terms_mutable_borrow_modification() {
        let mut atom = Atom::new(AtomSkeletonId::from(1), setup_dummy_terms());

        if let Some(term) = atom.terms_mut().get_mut(0) {
            *term = Term::Variable(VariableId::from(99));
        }

        assert_eq!(atom.terms()[0], Term::Variable(VariableId::from(99)));
    }

    /// **Objective**: Validate the `Display` string formatting for both standard predicates, standard negated predicates, and built-in equalities.
    ///
    /// **Input**: A standard atom, its negated version, and an equality atom.
    ///
    /// **Expected Output**: Standard outputs match `"sk_5(?v#0)"`, `"not(sk_5(?v#0))"`, and equality outputs clean up the built-in zone index to 0: `"sk_0(?v#0, ?v#1)"`.
    #[test]
    fn test_atom_display_formatting() {
        // 1. Prédicat classique positif et négatif
        let sk_id = AtomSkeletonId::from(5);
        let mut atom = Atom::new(sk_id, setup_dummy_terms());
        assert_eq!(format!("{}", atom), "sk_5(?v#0)");

        atom.negated();
        assert_eq!(format!("{}", atom), "not(sk_5(?v#0))");

        // 2. Égalité et Inégalité (L'index purifié .as_usize() doit valoir 0)
        let t1 = Term::Variable(VariableId::from(0));
        let t2 = Term::Variable(VariableId::from(1));
        let mut eq_atom = Atom::equality(t1, t2);
        assert_eq!(format!("{}", eq_atom), "sk_0(?v#0, ?v#1)");

        eq_atom.negated();
        assert_eq!(format!("{}", eq_atom), "not(sk_0(?v#0, ?v#1))");
    }

    /// **Objective**: Verify that modifying an atom's skeleton ID dynamically updates its equality status.
    ///
    /// **Input**: A standard atom mutated to `EQUALITY_ID`, and an equality atom mutated to a standard ID.
    ///
    /// **Expected Output**: `is_equality()` dynamically switches between `true` and `false` based on the assigned ID.
    #[test]
    fn test_set_skeleton_id_equality_impact() {
        let mut atom = Atom::new(AtomSkeletonId::from(5), setup_dummy_terms());
        assert!(!atom.is_equality());

        // Dynamic transition: standard atom becomes an equality constraint
        atom.set_skeleton_id(AtomSkeletonId::from(Atom::EQUALITY_ID));
        assert!(atom.is_equality());

        // Dynamic transition: equality constraint reverts to a standard atom
        atom.set_skeleton_id(AtomSkeletonId::from(42));
        assert!(!atom.is_equality());
    }

    /// **Objective**: Ensure the equality constructor safely handles reflexivity with identical terms.
    ///
    /// **Input**: `t1 = Term::Variable(0)` passed as both the left-hand and right-hand side arguments.
    ///
    /// **Expected Output**: A valid binary equality atom where both terms reference the exact same structural variable.
    #[test]
    fn test_equality_reflexivity_with_identical_terms() {
        let t1 = Term::Variable(VariableId::from(0));
        let atom = Atom::equality(t1.clone(), t1);

        assert!(atom.is_equality());
        assert_eq!(atom.arity(), 2);
        assert_eq!(atom.terms()[0], atom.terms()[1]);
    }
}
