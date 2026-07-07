use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use smallvec::SmallVec;
use std::fmt;

/// Represents a Datalog rule in the form `Head :- Body.`.
///
/// A rule defines a logical implication where the `head` atom is derived as a
/// new fact if all atoms in the `body` (the conjunction of premises) are
/// satisfied by the current state of the database.
///
/// # Memory Layout & Performance
///
/// To optimize the grounding engine's performance and prevent heap fragmentation
/// during rule transformation and join reordering phases, the rule's body utilizes
/// a stack-allocated [`SmallVec`]. Since most PDDL action translations generate rules
/// with a tight set of preconditions and typing literals, an inline capacity of
/// [`Rule::INLINE_BODY_CAPACITY`] (typically 8) ensures that the vast majority of rules
/// require **zero heap allocations** for their body components.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The conclusion or the fact produced by this rule.
    head: Atom,
    /// The list of atoms that must be satisfied to trigger the rule,
    /// optimized for stack-allocation.
    body: SmallVec<[Atom; Rule::INLINE_BODY_CAPACITY]>,
}

impl Rule {
    /// Threshold capacity for stack-allocated inline body storage.
    ///
    /// Rules with a body containing fewer than or equal to this number of preconditions
    /// will reside entirely on the stack, bypassing the heap allocator.
    pub const INLINE_BODY_CAPACITY: usize = 8;

    /// Creates a new Datalog rule.
    ///
    /// # Arguments
    ///
    /// * `head` - The conclusion [`Atom`] that will be inferred.
    /// * `body` - A standard vector of [`Atom`] preconditions representing the rule's body.
    ///
    /// # Return Value
    ///
    /// Returns a new instance of [`Self`] with the body safely migrated to an inline or heap-spilled `SmallVec`.
    #[inline]
    pub fn new(head: Atom, body: Vec<Atom>) -> Self {
        Self {
            head,
            body: SmallVec::from_vec(body),
        }
    }

    /// Returns a shared reference to the rule's head (conclusion).
    ///
    /// # Return Value
    ///
    /// Returns an immutable reference to the head [`Atom`].
    #[inline]
    pub fn head(&self) -> &Atom {
        &self.head
    }

    /// Returns a shared slice reference over the rule's body (conditions).
    ///
    /// # Return Value
    ///
    /// Returns an immutable slice reference (`&[Atom]`) targeting the underlying sequence of preconditions.
    #[inline]
    pub fn body(&self) -> &[Atom] {
        &self.body
    }

    /// Provides exclusive mutable access to the internal `SmallVec` container of the rule's body.
    ///
    /// This method is strictly restricted to the crate level (`pub(crate)`). It is primarily
    /// intended for the grounding engine to perform in-place heuristic optimizations (such as
    /// greedy static/dynamic join reordering) without exposing mutable internals to public callers.
    ///
    /// # Return Value
    ///
    /// Returns an exclusive mutable reference (`&mut SmallVec<...>`) over the rule's body.
    #[inline]
    pub(crate) fn body_mut(&mut self) -> &mut SmallVec<[Atom; Rule::INLINE_BODY_CAPACITY]> {
        &mut self.body
    }
}

impl fmt::Display for Rule {
    /// Formats the rule using standard Datalog/Prolog syntactic structures.
    ///
    /// Facts (rules with an empty body) are printed as just the head followed by a trailing period.
    /// Rules carrying premises use the classical `:-` separator block.
    ///
    /// # Examples
    ///
    /// * Fact signature: `sk_5(?v0).`
    /// * Rule signature: `sk_10(?v0) :- sk_1(?v0), sk_2(?v1).`
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` if formatting completes successfully, or a [`fmt::Error`] upon stream buffer failure.
    ///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;

        if !self.body.is_empty() {
            write!(f, " :- ")?;
            for (i, atom) in self.body.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", atom)?;
            }
        }

        write!(f, ".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, VariableId};

    /// Helper function to quickly instantiate a dummy positive Atom for testing rules.
    fn create_dummy_atom(id: usize) -> Atom {
        let sk_id = AtomSkeletonId::from(id);
        let terms = vec![Term::Variable(VariableId::from(0))];
        Atom::new(sk_id, terms)
    }

    /// **Objective**: Verify that a new Datalog rule can be correctly initialized from a head atom and a vector of body atoms.
    ///
    /// **Input**: A valid head `Atom` and a `Vec<Atom>` containing 2 elements.
    ///
    /// **Expected Output**: A `Rule` instance where `head()` matches the input head, `body()` contains exactly 2 atoms, and the data is stored inline on the stack.
    #[test]
    fn test_rule_creation_and_inline_storage() {
        let head = create_dummy_atom(10);
        let body_atoms = vec![create_dummy_atom(1), create_dummy_atom(2)];

        let rule = Rule::new(head, body_atoms);

        assert_eq!(rule.head().skeleton_id().as_usize(), 10);
        assert_eq!(rule.body().len(), 2);
        assert_eq!(rule.body()[0].skeleton_id().as_usize(), 1);
        assert_eq!(rule.body()[1].skeleton_id().as_usize(), 2);

        // Ensure that it fits within the stack allocation limit (Inline capacity is 8)
        assert!(!rule.body.spilled());
    }

    /// **Objective**: Verify that the rule's internal `SmallVec` triggers heap spillover when the body size exceeds `INLINE_BODY_CAPACITY`.
    ///
    /// **Input**: A rule initialized with a body sequence containing 9 atoms (exceeding the threshold limit of 8).
    ///
    /// **Expected Output**: The rule successfully holds all 9 elements, but `spilled()` evaluates to true, indicating a heap allocation transition.
    #[test]
    fn test_rule_body_heap_spillover() {
        let head = create_dummy_atom(100);
        let large_body = (0..=Rule::INLINE_BODY_CAPACITY)
            .map(create_dummy_atom)
            .collect::<Vec<_>>();

        let rule = Rule::new(head, large_body);

        assert_eq!(rule.body().len(), Rule::INLINE_BODY_CAPACITY + 1);
        assert!(
            rule.body.spilled(),
            "The rule body must migrate to the heap when arity > 8"
        );
    }

    /// **Objective**: Ensure that `body_mut()` provides direct exclusive access to the underlying container, allowing in-place mutations.
    ///
    /// **Input**: A rule with 2 body conditions, modified by swapping out the first atom completely.
    ///
    /// **Expected Output**: The internal storage is modified in place, and the first body atom updates to the newly assigned atom ID.
    #[test]
    fn test_rule_body_mutable_access() {
        let head = create_dummy_atom(1);
        let mut rule = Rule::new(head, vec![create_dummy_atom(10), create_dummy_atom(20)]);

        // Modify the body via the pub(crate) mutable reference
        {
            let body_ref = rule.body_mut();
            body_ref[0] = create_dummy_atom(99);
        }

        assert_eq!(rule.body()[0].skeleton_id().as_usize(), 99);
        assert_eq!(rule.body()[1].skeleton_id().as_usize(), 20);
    }

    /// **Objective**: Validate the standard Datalog string formatting output for both standard rules and fact structures.
    ///
    /// **Input**: A classical conditional rule instance and a separate fact instance (a rule with an empty body).
    ///
    /// **Expected Output**: The conditional rule formats cleanly as `"sk_10(?v#0) :- sk_1(?v#0), sk_2(?v#0)."` and the fact drops the implication arrow to render as `"sk_5(?v#0)."`.
    #[test]
    fn test_rule_display_formatting() {
        // 1. Test standard rule with preconditions
        let head = create_dummy_atom(10);
        let body = vec![create_dummy_atom(1), create_dummy_atom(2)];
        let rule = Rule::new(head, body);

        let expected_rule_output = "sk_10(?v#0) :- sk_1(?v#0), sk_2(?v#0).";
        assert_eq!(format!("{}", rule), expected_rule_output);

        // 2. Test fact representation (empty body)
        let fact_head = create_dummy_atom(5);
        let fact = Rule::new(fact_head, vec![]);

        let expected_fact_output = "sk_5(?v#0).";
        assert_eq!(format!("{}", fact), expected_fact_output);
    }

    /// **Objective**: Ensure that a rule can be successfully initialized with an empty body,
    /// representing a pure logical fact.
    ///
    /// **Input**: A valid head `Atom` and an empty `Vec<Atom>`.
    ///
    /// **Expected Output**: A `Rule` instance where `body()` has a length of 0, does not spill to the heap,
    /// and safely coexists with typical rule evaluation paths.
    #[test]
    fn test_rule_with_empty_body_fact() {
        let head = create_dummy_atom(5);
        let rule = Rule::new(head, vec![]);

        assert_eq!(rule.head().skeleton_id().as_usize(), 5);
        assert_eq!(rule.body().len(), 0);
        assert!(!rule.body.spilled());
    }

    /// **Objective**: Verify that the rule structure accurately preserves exactly `INLINE_BODY_CAPACITY` atoms
    /// on the stack without spilling to the heap.
    ///
    /// **Input**: A rule initialized with exactly 8 body conditions (the precise upper boundary limit).
    ///
    /// **Expected Output**: The rule's body has a length of 8, and `spilled()` remains strictly false,
    /// validating that boundary saturation maximizes stack usage.
    #[test]
    fn test_rule_body_exact_capacity_boundary() {
        let head = create_dummy_atom(77);
        let exact_body = (0..Rule::INLINE_BODY_CAPACITY)
            .map(create_dummy_atom)
            .collect::<Vec<_>>();

        let rule = Rule::new(head, exact_body);

        assert_eq!(rule.body().len(), Rule::INLINE_BODY_CAPACITY);
        assert!(
            !rule.body.spilled(),
            "An exact capacity of 8 must remain inline on the stack"
        );
    }

    /// **Objective**: Guarantee that clone operations deep-copy both the head and the entire body sequence
    /// regardless of whether the `SmallVec` layout is inline or spilled.
    ///
    /// **Input**: A rule containing a spilled heap allocation (9 atoms) cloned into a separate variable.
    ///
    /// **Expected Output**: The cloned instance contains identical data structures, structural equality holds,
    /// and modifying the original rule's body does not mutate the cloned rule's data state.
    #[test]
    fn test_rule_deep_cloning_behavior() {
        let head = create_dummy_atom(10);
        let large_body = (0..=Rule::INLINE_BODY_CAPACITY)
            .map(create_dummy_atom)
            .collect::<Vec<_>>();

        let mut original_rule = Rule::new(head, large_body);
        let cloned_rule = original_rule.clone();

        // Structural verification
        assert_eq!(cloned_rule.body().len(), original_rule.body().len());
        assert_eq!(
            cloned_rule.head().skeleton_id().as_usize(),
            original_rule.head().skeleton_id().as_usize()
        );
        assert!(cloned_rule.body.spilled());

        // Isolation mutation check
        original_rule.body_mut()[0] = create_dummy_atom(999);
        assert_ne!(
            cloned_rule.body()[0].skeleton_id().as_usize(),
            999,
            "Cloned data must be deep-copied and isolated"
        );
    }
}
