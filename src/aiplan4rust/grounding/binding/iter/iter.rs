use std::fmt;
use crate::aiplan4rust::grounding::binding::Bindings;
use crate::aiplan4rust::grounding::binding::iter::BindingsIteratorError;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{ObjectId, TypeId, TypedList, VariableId};
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;

/// A lazy combination iterator designed for exploring variable value domains.
///
/// `BindingsIterator` traverses the Cartesian product of multiple [`ValueDomain`]s.
/// It is optimized for grounding by allowing dynamic "pruning" of specific branches
/// in the search space, avoiding unnecessary computations of invalid combinations.
pub struct BindingsIterator<'a> {
    /// The variable identifiers associated with each domain in the iterator.
    variables: &'a TypedList<VariableId, TypeId>,

    /// The source value domains for each parameter, ordered by variable.
    domains: Vec<&'a ValueDomain>,

    /// Current cursor positions within each domain.
    indices: Vec<usize>,

    /// Internal buffer used to expose the current combination without re-allocating.
    current_bindings: Bindings,

    /// A flag indicating whether the entire search space has been traversed.
    exhausted: bool,

    /// The total number of possible combinations in the Cartesian product.
    total_count: usize,

    /// Internal state to handle the first iteration correctly.
    is_first: bool,
}

impl<'a> BindingsIterator<'a> {
    /// Initializes the iterator. Returns an exhausted iterator if the variables list
    /// or any of the domains are empty.
    ///
    /// # Parameters
    /// - `variables`: The list of typed variables to instantiate.
    /// - `value_registry`: The registry used to retrieve domains for each variable.
    ///
    /// # Returns
    /// - A new `BindingsIterator` instance or a `BindingsIteratorError` if an overflow occurs.
    pub fn new(variables: &'a TypedList<VariableId, TypeId>, value_registry: &'a ValueRegistry) -> Result<Self, BindingsIteratorError> {
        let domains = value_registry.get_variable_domains(variables);
        let arity = domains.len();

        // 1. Calculate the total number of combinations.
        // If arity is 0 (no variables), total_count is forced to 0 to trigger immediate exhaustion.
        let total_count = match arity {
            0 => 0,
            _ => domains
                .iter()
                .map(|d| d.cardinality())
                .try_fold(1usize, |acc, x| acc.checked_mul(x))
                .ok_or_else(|| BindingsIteratorError::combinatorial_explosion(arity))?,
        };

        // 2. Vacuity check (Grounding-specific logic).
        // The iterator is considered exhausted if:
        // - There are no variables to instantiate (arity == 0)
        // - The total combination count is 0
        // - Any of the required domains is empty
        let is_domain_empty = arity > 0 && domains.iter().any(|d| d.is_empty());
        let exhausted = arity == 0 || total_count == 0 || is_domain_empty;

        // 3. Buffer initialization
        let indices = vec![0; arity];

        // Pre-fill current_bindings with default ObjectIds to avoid allocations during iteration.
        let mut current_bindings = Bindings::with_capacity(arity);
        for typed_var in variables {
            current_bindings.insert(typed_var.symbol(), ObjectId::default());
        }

        Ok(Self {
            domains,
            variables,
            indices,
            current_bindings,
            exhausted,
            total_count,
            is_first: true,
        })
    }

    /// Returns the current combination and advances the iterator to the next state.
    ///
    /// This implementation is optimized to minimize bounds checks and avoids
    /// re-allocating the result by updating an internal buffer.
    ///
    /// # Returns
    /// - `Some(&Bindings)`: A reference to the current set of variable-to-object mappings.
    /// - `None`: If all combinations have been exhausted.
    pub fn next(&mut self) -> Option<&Bindings> {
        if self.exhausted {
            return None;
        }

        let arity = self.variables.len();

        // Case: First iteration
        if self.is_first {
            self.is_first = false;

            // Fill the buffer only if variables exist.
            // For arity 0, we return the empty Bindings buffer already initialized.
            if arity > 0 {
                for i in 0..arity {
                    let val = self.domains[i].get_value_at(0);
                    self.current_bindings.insert(self.variables[i].symbol(), val);
                }
            } else {
                // For arity 0, mark as exhausted immediately after this first yield.
                self.exhausted = true;
            }
        } else {
            // Subsequent calls (only valid if arity > 0)
            if arity == 0 {
                self.exhausted = true;
                return None;
            }

            // Advance indices and find the first level that changed.
            let changed_idx = self.prepare_next()?;

            // Update the buffer from the changed index onwards to synchronize with new indices.
            for i in changed_idx..arity {
                let val = self.domains[i].get_value_at(self.indices[i]);
                self.current_bindings.insert(self.variables[i].symbol(), val);
            }
        }

        Some(&self.current_bindings)
    }

    /// Increments the iteration indices to the next valid combination.
    ///
    /// This logic performs a lexicographical increment (similar to an odometer).
    /// When an index reaches the maximum cardinality of its domain, it wraps
    /// around to zero and carries over the increment to the next position to the left.
    ///
    /// # Returns
    /// - `Some(usize)`: The leftmost index that was incremented. This indicates that
    ///   all values to the left of this index remain unchanged, allowing for optimized buffer updates.
    /// - `None`: If the indices wrap around completely, indicating that all combinations
    ///   have been traversed.
    fn prepare_next(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            self.exhausted = true;
            return None;
        }

        // Iterate backwards from the rightmost domain (least significant position)
        for i in (0..self.indices.len()).rev() {
            if self.indices[i] + 1 < self.domains[i].cardinality() {
                self.indices[i] += 1;

                // Return 'i': signifies that all indices to the left of 'i' are unchanged.
                return Some(i);
            }

            // Reset current index and carry over to the next position on the left
            self.indices[i] = 0;
        }

        // Wrap around occurred for all domains
        self.exhausted = true;
        None
    }

    /// PRUNING: Skips all future combinations for positions to the right of `pos`.
    ///
    /// Calling `skip_at(1)` tells the iterator: "Move to the next value for index 1,
    /// ignoring all remaining possibilities for indices 2, 3, etc."
    ///
    /// # Parameters
    /// - `pos`: The index at which the pruning occurs. All domains to the right of this
    ///   position will be fast-forwarded to their last element.
    pub fn skip_at(&mut self, pos: usize) {
        let arity = self.indices.len();
        if pos < arity {
            // Optimization: We only iterate over the suffix to the right of pos.
            // This avoids re-evaluating the entire index set during the next increment.
            let suffix_start = pos + 1;
            if suffix_start < arity {
                for i in suffix_start..arity {
                    // By setting the indices to their maximum cardinality minus one,
                    // we force a "carry-over" effect during the next call to `prepare_next()`.
                    self.indices[i] = self.domains[i].cardinality().saturating_sub(1);
                }
            }
        }
    }

    /// Indicates whether the iterator has more combinations to yield.
    ///
    /// # Returns
    /// - `true` if there are remaining combinations, `false` if the iterator is exhausted.
    pub fn has_next(&self) -> bool {
        !self.exhausted
    }

    /// Resets the iterator to its initial state.
    ///
    /// This allows the same iterator to be reused for a fresh traversal of the
    /// Cartesian product without re-allocating the internal buffers or domains.
    pub fn reset(&mut self) {
        // 1. Reset all indices to the first element of each domain.
        self.indices.fill(0);

        // 2. Set state back to "first iteration" so the next call to next()
        // properly initializes the buffer.
        self.is_first = true;

        // 3. Re-evaluate the exhaustion state (identical to constructor logic).
        let arity = self.indices.len();
        let is_really_empty = arity > 0 && self.domains.iter().any(|d| d.is_empty());

        // Note: total_count is cached, so we don't need to re-check for overflows.
        self.exhausted = self.total_count == 0 || is_really_empty;
    }

    /// Returns the total number of possible combinations calculated at initialization.
    ///
    /// # Returns
    /// - The total count of combinations in the Cartesian product as a `usize`.
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Returns the number of combinations remaining to be traversed.
    ///
    /// The calculation has a complexity of `O(N)`, where `N` is the arity.
    /// This remains highly efficient (nearly instantaneous) even when the
    /// total number of combinations reaches millions.
    ///
    /// # Returns
    /// - The estimated count of remaining combinations as a `usize`.
    /// - Returns `0` if the iterator is already exhausted.
    pub fn remaining_count(&self) -> usize {
        if self.exhausted {
            return 0;
        }

        let mut remaining = 0usize;
        let mut multiplier = 1usize;

        // We traverse backwards (from least significant to most significant domain)
        // to calculate the remaining distance in the Cartesian product space.
        for i in (0..self.indices.len()).rev() {
            let card = self.domains[i].cardinality();
            let current_idx = self.indices[i];

            // Calculate how many elements are left in the current domain
            let diff = card.saturating_sub(1).saturating_sub(current_idx);

            // Accumulate the weighted remaining count
            remaining = remaining.saturating_add(diff.saturating_mul(multiplier));

            // Update the multiplier for the next (more significant) dimension.
            // Saturating multiplication prevents panics on theoretical overflows.
            multiplier = multiplier.saturating_mul(card);
        }

        // Add 1 to include the current combination in the count
        remaining.saturating_add(1)
    }
}

impl<'a> fmt::Display for BindingsIterator<'a> {
    /// Formats the iterator for display purposes, showing grounding progress.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of the formatting operation.
    ///
    /// The output provides a human-readable status including:
    /// - The number of combinations processed vs. the total count.
    /// - A percentage-based progress indicator.
    /// - The arity (number of variables) of the iterator.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.total_count().saturating_sub(self.remaining_count());
        let total = self.total_count();

        // Calculate progress percentage
        let progress = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        write!(
            f,
            "BindingsIterator [Progress: {}/{} ({:.2}%) | Arity: {}]",
            current,
            total,
            progress,
            self.indices.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{Type, TypedSymbol, VariableId, TypeId, ObjectId, TypedList};

    /// Helper to create a list of typed objects and populate the value registry.
    ///
    /// This function generates a sequence of object identifiers, wraps them
    /// as [`TypedSymbol`]s with a primitive type, and registers them into
    /// a new [`ValueRegistry`].
    ///
    /// # Parameters
    /// - `type_id`: The type identifier to assign to all created objects.
    /// - `num_objs`: The number of objects to generate (indexed from 0 to `num_objs - 1`).
    ///
    /// # Returns
    /// - A `ValueRegistry` containing the newly created typed objects, ready for domain extraction.
    fn create_registry_with_objects(type_id: TypeId, num_objs: usize) -> ValueRegistry {
        let objects: Vec<TypedSymbol<ObjectId, TypeId>> = (0..num_objs)
            .map(|i| {
                TypedSymbol::new(
                    ObjectId::from(i),
                    Type::primitive(type_id)
                )
            })
            .collect();

        ValueRegistry::new().with_typed_list(TypedList::from(objects))
    }

    #[test]
    // =========================================================================
    // TEST DESCRIPTION
    // -------------------------------------------------------------------------
    // OBJECTIVE: Verify the basic Cartesian product generation using a
    //            ValueRegistry to resolve variable domains.
    //
    // INPUT:
    //         - 2 variables sharing the same TypeId.
    //         - A registry containing 2 objects for that TypeId.
    //
    // EXPECTED OUTPUT:
    //         - A total count of 4 combinations (2 objects ^ 2 variables).
    //         - Each combination must contain exactly 2 valid bindings.
    // =========================================================================
    fn test_iterator_basic_product_with_registry() {
        // 1. Use a single TypeId for simplicity
        let tid = TypeId::from(0);

        // 2. Create 2 variables of type 'tid'
        let vars = TypedList::from(vec![
            TypedSymbol::new(VariableId::from(0), Type::primitive(tid)),
            TypedSymbol::new(VariableId::from(1), Type::primitive(tid)),
        ]);

        // 3. Create a registry with 2 objects for this type.
        // The domain for 'tid' will be [Obj0, Obj1].
        let registry = create_registry_with_objects(tid, 2);

        // 4. Initialization (the iterator will fetch the domain for 'tid' twice)
        let mut it = BindingsIterator::new(&vars, &registry).expect("Should initialize");

        // 2 objects ^ 2 variables = 4 combinations
        assert_eq!(it.total_count(), 4);

        let mut count = 0;
        while let Some(bindings) = it.next() {
            count += 1;
            assert_eq!(bindings.len(), 2);

            // Ensure both variables are correctly bound in the resulting Bindings object
            assert!(bindings.is_bound(&VariableId::from(0)));
            assert!(bindings.is_bound(&VariableId::from(1)));
        }

        // Verify that all 4 expected combinations were visited
        assert_eq!(count, 4);
    }

    #[test]
    // =========================================================================
    // TEST DESCRIPTION
    // -------------------------------------------------------------------------
    // OBJECTIVE: Verify that the iterator correctly maps and returns specific
    //            ObjectIds from the domain for a single variable.
    //
    // INPUT:
    //         - 1 variable of type 'tid'.
    //         - A registry containing 2 objects: [ObjectId(0), ObjectId(1)].
    //
    // EXPECTED OUTPUT:
    //         - The first call to next() must bind the variable to ObjectId(0).
    //         - The second call to next() must bind the variable to ObjectId(1).
    //         - The third call must return None.
    // =========================================================================
    fn test_iterator_direct_constants() {
        let tid = TypeId::from(0);

        // 1. Setup a single variable for tracking
        let vars = TypedList::from(vec![
            TypedSymbol::new(VariableId::from(0), Type::primitive(tid)),
        ]);

        // 2. Populate the registry with exactly 2 objects
        let registry = create_registry_with_objects(tid, 2);

        // 3. Initialize the iterator
        let mut it = BindingsIterator::new(&vars, &registry).unwrap();
        let var0 = VariableId::from(0);

        // 4. Validate the specific sequence of generated bindings
        // First iteration: expected ObjectId(0)
        assert_eq!(it.next().unwrap().get(&var0), Some(ObjectId::from(0)));

        // Second iteration: expected ObjectId(1)
        assert_eq!(it.next().unwrap().get(&var0), Some(ObjectId::from(1)));

        // End of domain reached
        assert!(it.next().is_none());
    }

    #[test]
    // =========================================================================
    // TEST DESCRIPTION
    // -------------------------------------------------------------------------
    // OBJECTIVE: Verify the "Pruning" (skip_at) logic. Ensure that skipping at
    //            a specific index correctly fast-forwards the odometer to the
    //            next value of the targeted variable, bypassing all remaining
    //            combinations of subsequent variables.
    //
    // INPUT:
    //         - 2 variables [Var0, Var1] sharing the same type.
    //         - A registry with 2 objects [Obj0, Obj1] (Domain size 2x2 = 4).
    //
    // EXPECTED OUTPUT:
    //         - Initial state: [Var0:0, Var1:0].
    //         - After skip_at(0): The iterator should skip [Var0:0, Var1:1]
    //           and land directly on [Var0:1, Var1:0].
    // =========================================================================
    fn test_skip_at_logic() {
        let tid = TypeId::from(0);

        // 1. Setup 2 variables for a 2D Cartesian product
        let vars = TypedList::from(vec![
            TypedSymbol::new(VariableId::from(0), Type::primitive(tid)),
            TypedSymbol::new(VariableId::from(1), Type::primitive(tid)),
        ]);
        let registry = create_registry_with_objects(tid, 2);

        let mut it = BindingsIterator::new(&vars, &registry).unwrap();
        let var0 = VariableId::from(0);
        let var1 = VariableId::from(1);

        // 2. Fetch the first combination: [Var0:0, Var1:0]
        it.next();

        // 3. Prune the branch at Var0 (index 0).
        // This logic sets all indices to the right of index 0 to their max value,
        // so the next increment pushes Var0 to its next value (1) and resets Var1 to (0).
        it.skip_at(0);

        // 4. Verify the jump
        // The iterator should have skipped [Var0:0, Var1:1]
        let res = it.next().expect("Should have element after skip");

        // Check that Var0 was incremented and Var1 was reset
        assert_eq!(res.get(&var0), Some(ObjectId::from(1)));
        assert_eq!(res.get(&var1), Some(ObjectId::from(0)));
    }

    #[test]
    // =========================================================================
    // TEST DESCRIPTION
    // -------------------------------------------------------------------------
    // OBJECTIVE: Verify the `reset()` functionality. Ensure that the iterator
    //            correctly restarts the traversal from the first combination
    //            without altering the pre-calculated `total_count`.
    //
    // INPUT:
    //         - 1 variable of type 'tid'.
    //         - A registry with 3 objects [Obj0, Obj1, Obj2].
    //
    // EXPECTED OUTPUT:
    //         - After advancing and calling reset(), the next value must be
    //           the first element of the domain again (Obj0).
    //         - The total_count must remain constant at 3.
    // =========================================================================
    fn test_reset_and_consistency() {
        let tid = TypeId::from(0);

        // 1. Setup a single variable for tracking
        let vars = TypedList::from(vec![
            TypedSymbol::new(VariableId::from(0), Type::primitive(tid)),
        ]);

        // 2. Populate the registry with 3 objects
        let registry = create_registry_with_objects(tid, 3);

        let mut it = BindingsIterator::new(&vars, &registry).unwrap();

        // 3. Partially traverse the iterator
        it.next(); // Yields Obj0
        it.next(); // Yields Obj1

        // 4. Perform the reset
        it.reset();

        // 5. Verify consistency
        // The total count should persist across resets
        assert_eq!(it.total_count(), 3);

        // The iterator must start over from the beginning of the domain
        let res = it.next().unwrap();
        assert_eq!(res.get(&VariableId::from(0)), Some(ObjectId::from(0)));
    }

    #[test]
    // =========================================================================
    // TEST DESCRIPTION
    // -------------------------------------------------------------------------
    // OBJECTIVE: Verify that the iterator handles the "empty variable list" case.
    //            An iterator with no variables to bind should be considered
    //            immediately exhausted.
    //
    // INPUT:
    //         - An empty `TypedList` of variables.
    //         - An empty `ValueRegistry`.
    //
    // EXPECTED OUTPUT:
    //         - `total_count` must be 0.
    //         - The first call to `next()` must return `None`.
    // =========================================================================
    fn test_iterator_empty_vars_is_none_immediately() {
        // 1. Setup with no variables
        let vars = TypedList::empty();
        let registry = ValueRegistry::new();

        // 2. Initialize the iterator
        let mut it = BindingsIterator::new(&vars, &registry).expect("Init failed");

        // 3. Verify that the iterator is treated as empty/exhausted
        assert_eq!(it.total_count(), 0);
        assert!(it.next().is_none(), "Should be None because there are no variables to instantiate.");
    }
}
