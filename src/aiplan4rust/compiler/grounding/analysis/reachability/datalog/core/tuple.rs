use crate::aiplan4rust::support::lang::ObjectId;
use crate::analysis::reachability::datalog::settings;
use smallvec::SmallVec;
use std::fmt;

/// A stack-allocated sequence of concrete object identifiers representing a ground tuple's arguments.
///
/// This specific layout is optimized via [`settings::INLINE_TUPLE_ARGS_CAPACITY`] to maximize
/// stack density and eliminate heap allocations during intense database insertion and evaluation loops.
pub type TupleArgs = SmallVec<[ObjectId; settings::INLINE_TUPLE_ARGS_CAPACITY]>;

/// Represents a concrete tuple stored within the Datalog database relations.
///
/// A tuple pairs a generic relation or definition identifier (`symbol`) with a sequential
/// list of fully instantiated ground arguments (`args`).
///
/// # Memory Layout & Performance
///
/// To prevent heap fragmentation during intensive database population and saturation loops,
/// the tuple's arguments use a stack-allocated [`TupleArgs`] container. Since the vast majority
/// of PDDL predicates and ground facts operate with a tight set of parameters, an inline
/// capacity of [`settings::INLINE_TUPLE_ARGS_CAPACITY`] guarantees **zero heap allocations**
/// for standard database entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tuple<ID> {
    /// The unique identifier of the definition or relation (e.g., `ActionDefId`, `AtomSkeletonId`).
    pub symbol: ID,
    /// The concrete ground arguments (instantiated objects) optimized for stack allocation.
    pub args: TupleArgs,
}

impl<ID> Tuple<ID> {
    /// Creates a new concrete ground tuple from an argument buffer.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The generic relation symbol or definition identifier.
    /// * `args` - A collection convertible into a [`TupleArgs`] container holding the concrete ground object identifiers.
    ///
    /// # Return Value
    ///
    /// Returns a new instance of [`Self`] with the arguments stored inline on the stack if possible.
    pub fn new<A>(symbol: ID, args: A) -> Self
    where
        A: Into<TupleArgs>,
    {
        Self {
            symbol,
            args: args.into(),
        }
    }

    /// Returns a copy of the relation or definition symbol identifier.
    ///
    /// # Return Value
    ///
    /// Returns the copied `ID` symbol token.
    pub fn symbol(&self) -> ID
    where
        ID: Copy,
    {
        self.symbol
    }

    /// Returns a shared slice reference over the tuple's ground arguments.
    ///
    /// # Return Value
    ///
    /// Returns an immutable slice reference (`&[ObjectId]`) targeting the sequence of object parameters.
    pub fn arguments(&self) -> &[ObjectId] {
        &self.args
    }

    /// Returns the arity (the number of instantiated arguments) of the tuple.
    ///
    /// # Return Value
    ///
    /// Returns a `usize` indicating the element count inside the underlying container.
    pub fn arity(&self) -> usize {
        self.args.len()
    }
}

impl<ID: fmt::Display> fmt::Display for Tuple<ID> {
    /// Formats the tuple using classic relational database notation.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` if formatting completes successfully, or a [`fmt::Error`] upon stream failure.
    ///
    /// # Examples
    ///
    /// Rendered signature: `pred_id(o#1, o#2)`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.symbol)?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::support::lang::AtomSkeletonId;
    use smallvec::smallvec;

    /// **Objective**: Verify that a new ground tuple can be correctly initialized, accessing its symbol and arguments natively.
    ///
    /// **Input**: A generic `AtomSkeletonId` symbol (ID 42) and a vector containing 2 concrete `ObjectId` elements.
    ///
    /// **Expected Output**: A `Tuple` instance where `symbol()` returns 42, `arity()` is exactly 2, and the underlying storage resides inline on the stack.
    #[test]
    fn test_tuple_creation_and_inline_storage() {
        let symbol = AtomSkeletonId::from(42);
        let ground_args: TupleArgs = smallvec![ObjectId::from(1), ObjectId::from(2)];

        let tuple = Tuple::new(symbol, ground_args);

        assert_eq!(tuple.symbol().as_usize(), 42);
        assert_eq!(tuple.arity(), 2);
        assert_eq!(tuple.arguments().len(), 2);
        assert_eq!(tuple.arguments()[0].as_usize(), 1);
        assert_eq!(tuple.arguments()[1].as_usize(), 2);

        // Ensures allocation limits are working as expected (Inline limit is settings::INLINE_TUPLE_ARGS_CAPACITY)
        assert!(!tuple.args.spilled(), "Arguments must remain on the stack");
    }

    /// **Objective**: Validate that the generic `Tuple` works perfectly with different symbol types (e.g., raw integers).
    ///
    /// **Input**: A simple numeric primitive `u32` as the symbol, and a vector of 3 `ObjectId` references.
    ///
    /// **Expected Output**: Structural layout is properly compiled and validated, verifying that `ID` bounds do not restrict primitive copying behavior.
    #[test]
    fn test_tuple_with_primitive_id_type() {
        let symbol: u32 = 999;
        let ground_args: TupleArgs =
            smallvec![ObjectId::from(10), ObjectId::from(20), ObjectId::from(30)];

        let tuple = Tuple::new(symbol, ground_args);

        assert_eq!(tuple.symbol(), 999);
        assert_eq!(tuple.arity(), 3);
        assert!(!tuple.args.spilled());
    }

    /// **Objective**: Verify that the arguments array safely triggers a heap spillover when its length exceeds `INLINE_TUPLE_ARGS_CAPACITY`.
    ///
    /// **Input**: A tuple initialized with concrete object arguments exceeding the stack limit.
    ///
    /// **Expected Output**: The tuple successfully instantiates, returns an `arity()` equal to `INLINE_TUPLE_ARGS_CAPACITY + 1`, and the internal `SmallVec` transitions `spilled()` to true.
    #[test]
    fn test_tuple_arguments_heap_spillover() {
        let symbol = AtomSkeletonId::from(100);
        let large_args: TupleArgs = (0..=settings::INLINE_TUPLE_ARGS_CAPACITY)
            .map(|id| ObjectId::from(id))
            .collect::<TupleArgs>();

        let tuple = Tuple::new(symbol, large_args);

        assert_eq!(tuple.arity(), settings::INLINE_TUPLE_ARGS_CAPACITY + 1);
        assert!(
            tuple.args.spilled(),
            "The tuple storage must migrate to the heap when arity exceeds {}",
            settings::INLINE_TUPLE_ARGS_CAPACITY
        );
    }

    /// **Objective**: Ensure that the structure handles boundary conditions perfectly when the argument slice size equals exactly `INLINE_TUPLE_ARGS_CAPACITY`.
    ///
    /// **Input**: A tuple holding exactly `INLINE_TUPLE_ARGS_CAPACITY` elements.
    ///
    /// **Expected Output**: The array size is evaluated properly, and `spilled()` remains strictly false, maximizing stack density.
    #[test]
    fn test_tuple_exact_capacity_boundary() {
        let symbol = AtomSkeletonId::from(7);
        let boundary_args = (0..settings::INLINE_TUPLE_ARGS_CAPACITY)
            .map(|id| ObjectId::from(id))
            .collect::<TupleArgs>();

        let tuple = Tuple::new(symbol, boundary_args);

        assert_eq!(tuple.arity(), settings::INLINE_TUPLE_ARGS_CAPACITY);
        assert!(
            !tuple.args.spilled(),
            "An exact capacity of {} must reside on the stack",
            settings::INLINE_TUPLE_ARGS_CAPACITY
        );
    }

    /// **Objective**: Check that empty tuples (relations with arity zero) compile and behave correctly without memory penalties.
    ///
    /// **Input**: A symbol combined with an empty vector of arguments.
    ///
    /// **Expected Output**: A valid `Tuple` with `arity() == 0`, which remains inline on the stack.
    #[test]
    fn test_empty_tuple_arity_zero() {
        let symbol = AtomSkeletonId::from(123);
        let tuple = Tuple::new(symbol, TupleArgs::new());

        assert_eq!(tuple.arity(), 0);
        assert!(!tuple.args.spilled());
    }

    /// **Objective**: Validate the relational formatting output generated by the `Display` trait implementation.
    ///
    /// **Input**: A standard generic tuple containing a mock `AtomSkeletonId` symbol and 2 arguments.
    ///
    /// **Expected Output**: The output matches the classic relational format standard string representation, matching the true `AS#` token style: `"AS#10(o#1, o#2)"`.
    #[test]
    fn test_tuple_display_formatting() {
        let symbol = AtomSkeletonId::from(10);
        let ground_args: TupleArgs = smallvec![ObjectId::from(1), ObjectId::from(2)];
        let tuple = Tuple::new(symbol, ground_args);

        // Formats using the true display prefix of AtomSkeletonId ("AS#")
        let expected_output = "AS#10(o#1, o#2)";
        assert_eq!(format!("{}", tuple), expected_output);
    }

    /// **Objective**: Guarantee that clone operations execute a true deep copy of the inner `SmallVec`
    /// arguments regardless of whether they are allocated on the stack or spilled to the heap.
    ///
    /// **Input**: A tuple with 2 arguments cloned into a new variable, followed by a direct evaluation.
    ///
    /// **Expected Output**: The cloned tuple is structurally identical and detached from the original instance.
    #[test]
    fn test_tuple_deep_cloning_behavior() {
        let symbol = AtomSkeletonId::from(5);
        let tuple_orig = Tuple::new(symbol, smallvec![ObjectId::from(1), ObjectId::from(2)]);
        let tuple_cloned = tuple_orig.clone();

        assert_eq!(tuple_cloned.symbol(), tuple_orig.symbol());
        assert_eq!(tuple_cloned.arity(), tuple_orig.arity());
        assert_eq!(tuple_cloned.arguments(), tuple_orig.arguments());
    }

    /// **Objective**: Verify that structural equality (`PartialEq`/`Eq`) and structural hashing (`Hash`)
    /// work perfectly, allowing tuples to be correctly processed inside collections like hash sets.
    ///
    /// **Input**: Two identical tuples and one distinct tuple with a different argument layout.
    ///
    /// **Expected Output**: Identical tuples must be equal and generate the exact same hash value,
    /// while the distinct tuple must fail the equality check.
    #[test]
    fn test_tuple_equality_and_hashing() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let t1 = Tuple::new(AtomSkeletonId::from(1), smallvec![ObjectId::from(10)]);
        let t2 = Tuple::new(AtomSkeletonId::from(1), smallvec![ObjectId::from(10)]);
        let t3 = Tuple::new(AtomSkeletonId::from(1), smallvec![ObjectId::from(20)]);

        // Test Equality
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);

        // Test Hash consistency
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        t1.hash(&mut h1);
        t2.hash(&mut h2);

        assert_eq!(
            h1.finish(),
            h2.finish(),
            "Identical tuples must yield identical hash values"
        );
    }
}
