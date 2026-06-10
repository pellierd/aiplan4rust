use crate::aiplan4rust::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::support::lang::{
    ObjectId, Type, TypeId, TypedList, TypedSymbol, VariableId,
};

/// Helper to create a list of typed objects and populate the value evaluator.
///
/// This function generates a sequence of object identifiers, wraps them
/// as [`TypedSymbol`]s with a primitive typing, and registers them into
/// a new [`ValueRegistry`].
///
/// # Parameters
/// - `type_id`: The typing identifier to assign to all created objects.
/// - `num_objs`: The number of objects to generate (indexed from 0 to `num_objs - 1`).
///
/// # Returns
/// - A `ValueRegistry` containing the newly created typed objects, ready for domain extraction.
fn create_registry_with_objects(type_id: TypeId, num_objs: usize) -> ValueRegistry {
    let objects: Vec<TypedSymbol<ObjectId, TypeId>> = (0..num_objs)
        .map(|i| TypedSymbol::new(ObjectId::from(i), Type::primitive(type_id)))
        .collect();

    // Utilisation du constructeur statique de test
    ValueRegistry::from_objects(objects)
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
//         - A evaluator containing 2 objects for that TypeId.
//
// EXPECTED OUTPUT:
//         - A total count of 4 combinations (2 objects ^ 2 variables).
//         - Each combination must contain exactly 2 valid bindings.
// =========================================================================
fn test_iterator_basic_product_with_registry() {
    // 1. Use a single TypeId for simplicity
    let tid = TypeId::from(0);

    // 2. Create 2 variables of typing 'tid'
    let vars = TypedList::from(vec![
        TypedSymbol::new(VariableId::from(0), Type::primitive(tid)),
        TypedSymbol::new(VariableId::from(1), Type::primitive(tid)),
    ]);

    // 3. Create a evaluator with 2 objects for this typing.
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
//         - 1 variable of typing 'tid'.
//         - A evaluator containing 2 objects: [ObjectId(0), ObjectId(1)].
//
// EXPECTED OUTPUT:
//         - The first call to next() must bind the variable to ObjectId(0).
//         - The second call to next() must bind the variable to ObjectId(1).
//         - The third call must return None.
// =========================================================================
fn test_iterator_direct_constants() {
    let tid = TypeId::from(0);

    // 1. Setup a single variable for tracking
    let vars = TypedList::from(vec![TypedSymbol::new(
        VariableId::from(0),
        Type::primitive(tid),
    )]);

    // 2. Populate the evaluator with exactly 2 objects
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
//         - 2 variables [Var0, Var1] sharing the same typing.
//         - A evaluator with 2 objects [Obj0, Obj1] (Domain size 2x2 = 4).
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
//         - 1 variable of typing 'tid'.
//         - A evaluator with 3 objects [Obj0, Obj1, Obj2].
//
// EXPECTED OUTPUT:
//         - After advancing and calling reset(), the next value must be
//           the first element of the domain again (Obj0).
//         - The total_count must remain constant at 3.
// =========================================================================
fn test_reset_and_consistency() {
    let tid = TypeId::from(0);

    // 1. Setup a single variable for tracking
    let vars = TypedList::from(vec![TypedSymbol::new(
        VariableId::from(0),
        Type::primitive(tid),
    )]);

    // 2. Populate the evaluator with 3 objects
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
    let registry = ValueRegistry::empty();

    // 2. Initialize the iterator
    let mut it = BindingsIterator::new(&vars, &registry).expect("Init failed");

    // 3. Verify that the iterator is treated as empty/exhausted
    assert_eq!(it.total_count(), 0);
    assert!(
        it.next().is_none(),
        "Should be None because there are no variables to instantiate."
    );
}
