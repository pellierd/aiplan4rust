use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use crate::analysis::reachability::datalog::settings;
use smallvec::SmallVec;
use std::fmt;

/// A stack-allocated sequence of terms representing the arguments of a Datalog atom.
pub type AtomArgs = SmallVec<[Term; settings::INLINE_ATOM_ARGS_CAPACITY]>;

/// Represents a logical atom within a Datalog rule or fact representation.
///
/// An `Atom` is the foundational relational component of the Datalog grounding engine.
/// It binds a specific relation or predicate **`symbol`** (identified by its [`AtomSkeletonId`])
/// to an ordered sequence of arguments ([`AtomArgs`]), which may encompass either variables or constants.
/// In the context of reachability analysis, atoms populate the heads and bodies of Horn clauses
/// to dictate how newly derived facts propagate through the database network.
///
/// Atoms function as schema-level templates that the engine evaluates, unifies, and translates
/// into concrete relational tuples when querying or updating the
/// [`Database`](crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database).
///
/// # Memory Layout & Performance
///
/// * **Packed Fields**: The [`AtomSkeletonId`] packed inside the `symbol` field encodes boolean control
///   flags (such as the negation status and built-in zone markers) directly into the unused upper bits
///   of the predicate index. This keeps the footprint lightweight and optimizes CPU cache locality.
/// * **Inline Stack Allocation**: To prevent continuous heap fragmentation during iterative grounding loops,
///   arguments are stored inside a stack-allocated [`AtomArgs`] container. For any relation with an
///   arity lower than or equal to [`settings::INLINE_ATOM_ARGS_CAPACITY`] (typically 4 or fewer), the structure
///   triggers **zero heap allocations**, guaranteeing extreme data locality and low cache-miss ratios.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Atom {
    /// The unique symbol identifier for the predicate signature (encompassing name, arity, and parameter types).
    /// This packed ID acts as the index key to locate and modify the matching relation block inside the database.
    symbol: AtomSkeletonId,
    /// The contiguous sequence of arguments assigned to this instance, matching either
    /// [`Term::Variable`] placeholders or evaluated [`Term::Constant`] literals.
    args: AtomArgs,
}

impl Atom {
    /// Offset defining the start of the built-in zone.
    /// Shifted to avoid interfering with the negation flag (bit 60) or sign bits.
    pub const BUILTIN_ZONE_START: usize = 1 << 62;

    /// Immutable identifier reserved for structural equality predicates.
    pub const EQUALITY_ID: usize = Self::BUILTIN_ZONE_START;

    /// Creates a new n-ary `Atom` from an internal stack-allocated buffer.
    ///
    /// By default, a newly constructed atom is initialized as a positive (non-negated) literal.
    ///
    /// # Performance Note
    ///
    /// For optimal performance and to guarantee zero heap allocations, pass a stack-allocated
    /// [`AtomArgs`] or a fixed-size array directly. Passing a standard [`Vec`] will move the
    /// allocation to the heap, bypassing the stack-inlining optimization even if the number of
    /// arguments is below [`settings::INLINE_ATOM_ARGS_CAPACITY`].
    ///
    /// # Arguments
    ///
    /// * `symbol` - The unique [`AtomSkeletonId`] relation symbol mapping to a specific predicate schema.
    /// * `args` - An [`AtomArgs`] container containing the variables or constants assigned to this predicate.
    ///
    /// # Return Value
    ///
    /// Returns a fresh [`Self`] instance wrapping the provided relation symbol and arguments.
    ///
    /// # Complexity
    ///
    /// Constant time O(1) auxiliary complexity. If the resolved collection contains fewer elements than or
    /// exactly matching [`settings::INLINE_ATOM_ARGS_CAPACITY`], the arguments are inlined directly on the stack,
    /// bypassing heap allocation entirely.
    pub fn nary(symbol: AtomSkeletonId, args: AtomArgs) -> Self {
        Self { symbol, args }
    }

    /// Creates a new unary `Atom` containing exactly one argument.
    ///
    /// This specialized constructor guarantees that the single argument is stored
    /// directly on the stack, ensuring zero heap allocations.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The unique [`AtomSkeletonId`] relation symbol mapping to a specific unary predicate schema.
    /// * `arg` - The single variable or constant assigned as the argument.
    ///
    /// # Return Value
    ///
    /// Returns a fresh [`Self`] instance initialized as a positive unary literal.
    pub fn unary(symbol: AtomSkeletonId, arg: Term) -> Self {
        let mut args = AtomArgs::new();
        args.push(arg);

        Self { symbol, args }
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
    /// Returns a binary [`Self`] instance configured with the reserved equality symbol.
    pub fn equality(t1: Term, t2: Term) -> Self {
        let mut args = AtomArgs::new();
        args.push(t1);
        args.push(t2);

        Self {
            symbol: AtomSkeletonId::from(Atom::EQUALITY_ID),
            args,
        }
    }

    /// Toggles the negation state of the atom in place.
    ///
    /// This method flips the negation control flag (typically bit 60) stored within the
    /// underlying relation [`AtomSkeletonId`]. If the atom was positive, it becomes negated
    /// (representing a negative literal or a delete effect), and vice versa.
    ///
    /// # Implementation Details
    ///
    /// The state inversion relies on reading the current bit flag via [`Self::is_negated()`],
    /// applying a logical NOT, and committing the updated state back to the bitfield.
    /// This prevents altering the base index or the built-in zone tracking bits.
    pub fn negated(&mut self) {
        let new_state = !self.is_negated();
        self.symbol.set_negated(new_state);
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
    /// The verification relies on comparing the predicate's symbol identifier against the
    /// reserved [`Self::EQUALITY_ID`]. Because a negated equality (an inequality) flips the
    /// sign bit (bit 60), a direct raw comparison would fail. To prevent this, a bitwise NOT
    /// combined with an AND mask (`& !(1 << 60)`) clears the negation flag before performing
    /// the equality check.
    pub fn is_equality(&self) -> bool {
        // Mask out the negation bit (bit 60) so that inequalities are also recognized.
        (self.symbol.as_raw_usize() & !(1 << 60)) == Atom::EQUALITY_ID
    }

    /// Checks whether this atom is negated by evaluating the Most Significant Bit (MSB) of its symbol identifier.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the atom carries the negation flag (bit 60 set to 1), representing a negative literal
    /// (e.g., `not(p(...))`), and `false` if it represents a positive literal.
    #[inline(always)]
    pub fn is_negated(&self) -> bool {
        self.symbol.is_negated()
    }

    /// Explicitly forces or clears the negation state of the atom by modifying its internal symbol flags.
    ///
    /// # Arguments
    ///
    /// * `negated` - A boolean flag where `true` marks the atom as negated and `false` restores it to positive.
    #[inline(always)]
    pub fn set_negated(&mut self, negated: bool) {
        self.symbol.set_negated(negated);
    }

    /// Returns the unique relation symbol of the atom.
    ///
    /// # Return Value
    ///
    /// Returns a copy of the [`AtomSkeletonId`] acting as the relation symbol.
    #[inline]
    pub fn symbol(&self) -> AtomSkeletonId {
        self.symbol
    }

    /// Updates the relation symbol identifier (predicate ID).
    ///
    /// This is primarily used during the encoding phase to transform a base
    /// predicate ID into a shifted ID (e.g., applying the `negation_offset`
    /// to represent a delete effect in the Datalog engine).
    ///
    /// # Arguments
    ///
    /// * `new_symbol` - The new [`AtomSkeletonId`] relation symbol to associate with this atom.
    #[inline]
    pub fn set_symbol(&mut self, new_symbol: AtomSkeletonId) {
        self.symbol = new_symbol;
    }

    /// Returns an immutable borrow of the arguments sequence assigned to this atom.
    ///
    /// # Return Value
    ///
    /// Returns a shared slice reference (`&[Term]`) representing the ordered arguments of the predicate.
    #[inline]
    pub fn arguments(&self) -> &[Term] {
        &self.args
    }

    /// Replaces the entire sequence of arguments in the atom with a new stack-allocated collection.
    ///
    /// While providing exclusive mutable access for in-place modifications (such as variable
    /// unification or aliasing) is often preferred, this method allows for a complete structural
    /// swap of the atom's arguments.
    ///
    /// # Performance Note
    ///
    /// For optimal performance and to preserve stack-allocation benefits, pass a stack-allocated
    /// [`AtomArgs`] directly. Passing a standard [`Vec`] will force the allocated elements to
    /// reside on the heap, bypassing the stack-inlining optimization even if the new number
    /// of arguments sits below [`settings::INLINE_ATOM_ARGS_CAPACITY`].
    ///
    /// # Arguments
    ///
    /// * `new_args` - An [`AtomArgs`] container holding the complete sequence of new [`Term`] arguments.
    #[inline]
    pub fn set_arguments(&mut self, new_args: AtomArgs) {
        self.args = new_args;
    }

    /// Returns a mutable slice of the atom's arguments.
    ///
    /// This is primarily used during the encoding phase for variable aliasing,
    /// unification logic, or grounding, allowing fast in-place modifications of parameters
    /// without reallocating the underlying buffer wrapper.
    ///
    /// # Return Value
    ///
    /// Returns an exclusive mutable slice reference (`&mut [Term]`) over the atom's arguments.
    #[inline]
    pub fn arguments_mut(&mut self) -> &mut [Term] {
        &mut self.args
    }

    /// Returns the arity (the number of arguments) of the atom.
    ///
    /// # Return Value
    ///
    /// Returns a `usize` indicating the size of the underlying arguments container.
    ///
    /// # Complexity
    ///
    /// Constant time O(1) since vector length tracking is managed on the stack.
    #[inline]
    pub fn arity(&self) -> usize {
        self.args.len()
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
    /// * **Time Complexity**: O(N) where N is the arity (number of arguments) of the atom,
    ///   as it must visit and format each individual argument.
    /// * **Memory Complexity**: O(1) auxiliary space, since it streams the formatted characters
    ///   directly into the provided formatter buffer without allocating dynamic string buffers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_neg = self.is_negated();

        if is_neg {
            write!(f, "not(")?;
        }

        // Use `.as_usize()` to safely mask out control bits (e.g., negation flag),
        // preventing corrupted identifiers like `not(not_AS#...)` in the output logs.
        write!(f, "sk_{}(", self.symbol.as_usize())?;

        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
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
    use smallvec::smallvec;

    /// Helper function to create a basic set of arguments for testing.
    /// Uses a concrete enum variant (`Term::Variable`) to bypass the lack of `Default`.
    fn setup_dummy_arguments() -> AtomArgs {
        smallvec![Term::Variable(VariableId::from(0))]
    }

    /// **Objective**: Verify that a new positive atom is correctly initialized with its relation symbol and arguments.
    ///
    /// **Input**: `symbol = 42`, `args = [Term::Variable(0)]`
    ///
    /// **Expected Output**: A positive (non-negated) Atom with an arity of 1 and relation symbol ID 42.
    #[test]
    fn test_new_atom_creation() {
        let symbol_id = AtomSkeletonId::from(42);
        let args = setup_dummy_arguments();
        let atom = Atom::nary(symbol_id, args.clone());

        assert_eq!(atom.symbol().as_usize(), 42);
        assert_eq!(atom.arity(), args.len());
        assert!(!atom.is_negated());
        assert!(!atom.is_equality());
    }

    /// **Objective**: Ensure that the built-in equality atom is created with the reserved `EQUALITY_ID` and holds exactly 2 arguments.
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
        assert_eq!(atom.symbol().as_raw_usize(), Atom::EQUALITY_ID);
        assert_eq!(atom.arity(), 2);
        assert!(!atom.is_negated());
    }

    /// **Objective**: Validate that toggling negation modifies the MSB/flag bit 60 without altering the underlying base predicate ID.
    ///
    /// **Input**: A fresh positive atom with relation symbol ID 10.
    ///
    /// **Expected Output**: Alternates between negated (true) and positive (false). `as_usize()` must always return 10.
    #[test]
    fn test_negation_toggling_via_msb() {
        let symbol_id = AtomSkeletonId::from(10);
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(symbol_id, args);

        assert!(!atom.is_negated());

        // First toggle: Positive -> Negated (Bit 60 turns to 1)
        atom.negated();
        assert!(atom.is_negated());
        assert_eq!(atom.symbol().as_usize(), 10);

        // Second toggle: Negated -> Positive (Bit 60 turns to 0)
        atom.negated();
        assert!(!atom.is_negated());
        assert_eq!(atom.symbol().as_usize(), 10);
    }

    /// **Objective**: Verify that forcing a specific negation state via `set_negated` behaves deterministically.
    ///
    /// **Input**: A fresh positive atom with relation symbol ID 15.
    ///
    /// **Expected Output**: Reflects the exact boolean state assigned, regardless of the previous state.
    #[test]
    fn test_explicit_set_negated() {
        let symbol_id = AtomSkeletonId::from(15);
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(symbol_id, args);

        atom.set_negated(true);
        assert!(atom.is_negated());

        atom.set_negated(true); // Redundant assignment
        assert!(atom.is_negated());

        // Restore
        atom.set_negated(false);
        assert!(!atom.is_negated());
    }

    /// **Objective**: Ensure that replacing the predicate relation symbol works correctly (crucial for encoding offsets).
    ///
    /// **Input**: An atom initialized with ID 5, mutated to ID 100.
    ///
    /// **Expected Output**: The atom's relation symbol changes to 100.
    #[test]
    fn test_set_symbol_mutation() {
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(AtomSkeletonId::from(5), args);
        atom.set_symbol(AtomSkeletonId::from(100));

        assert_eq!(atom.symbol().as_usize(), 100);
    }

    /// **Objective**: Check that replacing the entire arguments sequence updates the atom's internal vector and arity.
    ///
    /// **Input**: An atom with 1 argument, updated with a new vector of 2 arguments.
    ///
    /// **Expected Output**: Internal arguments are replaced, and arity updates from 1 to 2.
    #[test]
    fn test_arguments_sequence_replacement() {
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(AtomSkeletonId::from(1), args);
        assert_eq!(atom.arity(), 1);

        let mut new_sequence = AtomArgs::new();
        new_sequence.push(Term::Variable(VariableId::from(0)));
        new_sequence.push(Term::Variable(VariableId::from(1)));

        atom.set_arguments(new_sequence);
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

    /// **Objective**: Ensure `arguments_mut` allows in-place modification of arguments without structural reallocation.
    ///
    /// **Input**: An atom with `Term::Variable(0)`, mutated to `Term::Variable(99)`.
    ///
    /// **Expected Output**: The argument at index 0 is successfully updated to 99.
    #[test]
    fn test_arguments_mutable_borrow_modification() {
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(AtomSkeletonId::from(1), args);

        if let Some(arg) = atom.arguments_mut().get_mut(0) {
            *arg = Term::Variable(VariableId::from(99));
        }

        assert_eq!(atom.arguments()[0], Term::Variable(VariableId::from(99)));
    }

    /// **Objective**: Validate the `Display` string formatting for both standard predicates, standard negated predicates, and built-in equalities.
    ///
    /// **Input**: A standard atom, its negated version, and an equality atom.
    ///
    /// **Expected Output**: Standard outputs match `"sk_5(?v#0)"`, `"not(sk_5(?v#0))"`, and equality outputs clean up the built-in zone index to 0: `"sk_0(?v#0, ?v#1)"`.
    #[test]
    fn test_atom_display_formatting() {
        let symbol_id = AtomSkeletonId::from(5);
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(symbol_id, args);
        assert_eq!(format!("{}", atom), "sk_5(?v#0)");

        atom.negated();
        assert_eq!(format!("{}", atom), "not(sk_5(?v#0))");

        let t1 = Term::Variable(VariableId::from(0));
        let t2 = Term::Variable(VariableId::from(1));
        let mut eq_atom = Atom::equality(t1, t2);
        assert_eq!(format!("{}", eq_atom), "sk_0(?v#0, ?v#1)");

        eq_atom.negated();
        assert_eq!(format!("{}", eq_atom), "not(sk_0(?v#0, ?v#1))");
    }

    /// **Objective**: Verify that modifying an atom's relation symbol dynamically updates its equality status.
    ///
    /// **Input**: A standard atom mutated to `EQUALITY_ID`, and an equality atom mutated to a standard ID.
    ///
    /// **Expected Output**: `is_equality()` dynamically switches between `true` and `false` based on the assigned ID.
    #[test]
    fn test_set_symbol_equality_impact() {
        let args = setup_dummy_arguments();
        let mut atom = Atom::nary(AtomSkeletonId::from(5), args);
        assert!(!atom.is_equality());

        // Dynamic transition: standard atom becomes an equality constraint
        atom.set_symbol(AtomSkeletonId::from(Atom::EQUALITY_ID));
        assert!(atom.is_equality());

        // Dynamic transition: equality constraint reverts to a standard atom
        atom.set_symbol(AtomSkeletonId::from(42));
        assert!(!atom.is_equality());
    }

    /// **Objective**: Ensure the equality constructor safely handles reflexivity with identical terms as arguments.
    ///
    /// **Input**: `t1 = Term::Variable(0)` passed as both the left-hand and right-hand side arguments.
    ///
    /// **Expected Output**: A valid binary equality atom where both arguments reference the exact same structural variable.
    #[test]
    fn test_equality_reflexivity_with_identical_terms() {
        let t1 = Term::Variable(VariableId::from(0));
        let atom = Atom::equality(t1.clone(), t1);

        assert!(atom.is_equality());
        assert_eq!(atom.arity(), 2);
        assert_eq!(atom.arguments()[0], atom.arguments()[1]);
    }

    /// **Objective**: Verify that the arguments buffer accurately switches from stack to heap allocation when arguments exceed `INLINE_ATOM_ARGS_CAPACITY`.
    ///
    /// **Input**: An atom initialized with elements sitting below the stack allocation threshold, then replaced with a long sequence.
    ///
    /// **Expected Output**: The initial layout must remain inline on the stack, but after swapping via `set_arguments` with an item count higher than the limit, `spilled()` must return true.
    #[test]
    fn test_atom_buffer_spillover_transition() {
        let symbol_id = AtomSkeletonId::from(1);

        // Explicit type verification to enforce stack allocation constraints via AtomArgs alias
        let initial_args: AtomArgs = smallvec![Term::Variable(VariableId::from(0))];
        let mut atom = Atom::nary(symbol_id, initial_args);

        // On inspecte directement le champ structurel privé `args` pour tester le SmallVec
        assert!(
            !atom.args.spilled(),
            "The atom must remain on the stack when below capacity"
        );

        // Exceeds the capacity threshold -> Switches to the heap
        let large_sequence: AtomArgs = (0..=settings::INLINE_ATOM_ARGS_CAPACITY)
            .map(|i| Term::Variable(VariableId::from(i)))
            .collect();

        atom.set_arguments(large_sequence);

        assert_eq!(atom.arity(), settings::INLINE_ATOM_ARGS_CAPACITY + 1);
        assert!(
            atom.args.spilled(),
            "The atom arguments must migrate to the heap when count > {}",
            settings::INLINE_ATOM_ARGS_CAPACITY
        );
    }
}
