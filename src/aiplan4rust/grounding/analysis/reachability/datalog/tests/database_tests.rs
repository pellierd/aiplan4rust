use crate::aiplan4rust::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId};
use std::error::Error;

/// Type alias for cleaner test signatures using the standard Error trait.
type TestResult = Result<(), Box<dyn Error>>;

/// # Objective
/// Verify that a fact added to the stable storage is correctly persisted and retrievable.
///
/// # Input
/// - Predicate ID: `1`
/// - Tuple of ObjectIds: `[10, 20]`
///
/// # Expected Output
/// - `contains_stable` returns `true`.
/// - `contains_delta` returns `false`.
#[test]
fn test_add_and_contains() {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);
    let fact = vec![ObjectId::from(10), ObjectId::from(20)];

    db.insert_stable_fact(sk_id, &fact);

    assert!(
        db.contains_stable(sk_id, &fact),
        "Fact should be confirmed in stable storage"
    );
    assert!(
        !db.contains_delta(sk_id, &fact),
        "Fact should not exist in the delta buffer"
    );
}

/// # Objective
/// Ensure the database maintains set semantics (uniqueness) to prevent infinite loops during saturation.
///
/// # Input
/// - Action: Adding the same fact `[1]` twice for the same predicate.
///
/// # Expected Output
/// - Total facts for the relation: `1` (duplicates must be ignored).
#[test]
fn test_no_duplicates() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);
    let fact = vec![ObjectId::from(1)];

    db.insert_stable_fact(sk_id, &fact);
    db.insert_stable_fact(sk_id, &fact);

    let (len, _) = db
        .get_layout(sk_id, false)
        .ok_or("Relation missing after insertion")?;

    assert_eq!(len, 1, "Database should only store unique facts");
    Ok(())
}

/// # Objective
/// Validate the indexing mechanism for fast lookup by the first argument (join optimization).
///
/// # Input
/// - Facts: `[1, 10]`, `[2, 20]`, `[1, 30]`
/// - Query: `get_offsets_for(first_arg = 1)`
///
/// # Expected Output
/// - A vector containing exactly `2` offsets pointing to the facts starting with `1`.
#[test]
fn test_indexing_offsets() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);

    db.insert_stable_fact(sk_id, &[ObjectId::from(1), ObjectId::from(10)]);
    db.insert_stable_fact(sk_id, &[ObjectId::from(2), ObjectId::from(20)]);
    db.insert_stable_fact(sk_id, &[ObjectId::from(1), ObjectId::from(30)]);

    let offsets = db
        .lookup_index(sk_id, false, ObjectId::from(1))
        .ok_or("Index for value '1' should have been created")?;

    assert_eq!(
        offsets.len(),
        2,
        "Index should find all facts matching the first argument"
    );
    Ok(())
}

/// # Objective
/// Confirm the 'Semi-Naive' lifecycle: New facts must stay in Delta until promoted to Stable.
///
/// # Input
/// - Step 1: `add_delta_fact([55])`.
/// - Step 2: `commit_delta()`.
///
/// # Expected Output
/// - Before commit: `contains_delta` is `true`.
/// - After commit: `contains_stable` is `true` and Delta is empty.
#[test]
fn test_semi_naive_cycle() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);
    let fact = vec![ObjectId::from(55)];

    db.insert_delta_fact(sk_id, &fact);
    assert!(
        db.contains_delta(sk_id, &fact),
        "Fact must be isolated in delta initially"
    );

    db.commit_delta();
    assert!(db.is_delta_empty(), "Delta must be flushed after commit");
    assert!(
        db.contains_stable(sk_id, &fact),
        "Fact must be promoted to the stable set"
    );

    Ok(())
}

/// # Objective
/// Ensure data integrity and correct memory offset calculation when mixing different arities.
///
/// # Input
/// - Unary fact: `[100]` (Arity 1)
/// - Ternary fact: `[100, 200, 300]` (Arity 3)
///
/// # Expected Output
/// - `fetch_tuple` on ternary relation at offset 0 returns `300` at index `2`.
#[test]
fn test_mixed_arities() -> TestResult {
    let mut db = Database::new();
    let sk_unary = AtomSkeletonId::from(1);
    let sk_ternary = AtomSkeletonId::from(2);

    db.insert_stable_fact(sk_unary, &[ObjectId::from(100)]);
    db.insert_stable_fact(
        sk_ternary,
        &[
            ObjectId::from(100),
            ObjectId::from(200),
            ObjectId::from(300),
        ],
    );

    let (_, arity1) = db.get_layout(sk_unary, false).ok_or("Unary missing")?;
    let (_, arity2) = db.get_layout(sk_ternary, false).ok_or("Ternary missing")?;

    assert_eq!(arity1, 1);
    assert_eq!(arity2, 3);

    let mut buffer = [ObjectId::from(0); 3];
    db.read_tuple(sk_ternary, false, 0, 3, &mut buffer);
    assert_eq!(
        buffer[2],
        ObjectId::from(300),
        "Should retrieve the correct argument from memory"
    );

    Ok(())
}

/// # Objective
/// Ensure the database handles queries for non-existent relations gracefully.
///
/// # Input
/// - Query `contains_stable` for a `SkeletonId` never added.
/// - Query `get_offsets_for` for a `SkeletonId` never added.
///
/// # Expected Output
/// - `contains_stable` returns `false` (no panic).
/// - `get_offsets_for` returns `None` (no panic).
#[test]
fn test_empty_database_queries() {
    let db = Database::new();
    let sk_id = AtomSkeletonId::from(999);
    let fact = vec![ObjectId::from(1)];

    assert!(
        !db.contains_stable(sk_id, &fact),
        "Should not contain facts in empty DB"
    );
    assert!(
        db.lookup_index(sk_id, false, ObjectId::from(1)).is_none(),
        "Offsets should be None"
    );
}

/// # Objective
/// Verify that `clear_all` effectively wipes all data from both Stable and Delta storages.
///
/// # Input
/// - Add a fact to Stable.
/// - Add a fact to Delta.
/// - Call `clear_all()`.
///
/// # Expected Output
/// - `total_facts_count` is `0`.
/// - `is_delta_empty` is `true`.
#[test]
fn test_clear_all() {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);

    db.insert_stable_fact(sk_id, &[ObjectId::from(10)]);
    db.insert_delta_fact(sk_id, &[ObjectId::from(20)]);

    db.clear_all();

    assert_eq!(db.total_facts_count(), 0, "Stable storage should be empty");
    assert!(db.is_delta_empty(), "Delta storage should be empty");
}

/// # Objective
/// Verify that `move_all_to_delta` correctly transfers the entire state for the initial bootstrap.
///
/// # Input
/// - Add initial facts (facts from the PDDL problem) to Stable.
/// - Call `move_all_to_delta()`.
///
/// # Expected Output
/// - Stable becomes empty.
/// - Delta contains exactly the facts previously in Stable.
#[test]
fn test_move_all_to_delta() {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);
    let fact = vec![ObjectId::from(100)];

    db.insert_stable_fact(sk_id, &fact);
    db.move_all_to_delta();

    assert!(
        db.get_relation(sk_id).is_none(),
        "Stable should be empty after move"
    );
    assert!(
        db.contains_delta(sk_id, &fact),
        "Fact should now be in Delta"
    );
}

/// # Objective
/// ensure that a fact is not added to Delta if it already exists in Stable.
/// This is crucial for the termination of the semi-naive algorithm.
///
/// # Input
/// - Add fact [10] to Stable.
/// - Try to add fact [10] to Delta.
///
/// # Expected Output
/// - `add_delta_fact` returns `false`.
/// - Delta remains empty.
#[test]
fn test_no_delta_if_already_in_stable() {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);
    let fact = vec![ObjectId::from(10)];

    db.insert_stable_fact(sk_id, &fact);
    let added = db.insert_delta_fact(sk_id, &fact);

    assert!(!added, "Should not add to delta if fact is already stable");
    assert!(db.is_delta_empty(), "Delta should remain empty");
}

/// # Objective
/// Handle predicates with arity 0 (boolean flags).
///
/// # Input
/// - Add an empty fact vector `[]`.
///
/// # Expected Output
/// - `contains_stable` returns `true`.
/// - `get_raw_info` reports arity 0.
#[test]
fn test_arity_zero() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(100);
    let empty_fact: Vec<ObjectId> = vec![];

    db.insert_stable_fact(sk_id, &empty_fact);

    assert!(db.contains_stable(sk_id, &empty_fact));
    let (len, arity) = db.get_layout(sk_id, false).ok_or("Missing relation")?;
    assert_eq!(arity, 0);
    assert_eq!(len, 0);
    Ok(())
}

/// # Objective
/// Verify that fetching tuples from Stable does not return data stored in Delta.
///
/// # Input
/// - Fact [10] in Stable.
/// - Fact [20] in Delta.
///
/// # Expected Output
/// - `fetch_tuple` on Stable (delta=false) at offset 0 must return [10].
/// - `fetch_tuple` on Delta (delta=true) at offset 0 must return [20].
#[test]
fn test_delta_isolation() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);

    db.insert_stable_fact(sk_id, &[ObjectId::from(10)]);
    db.insert_delta_fact(sk_id, &[ObjectId::from(20)]);

    let mut buffer = [ObjectId::from(0); 1];

    // Check Stable
    db.read_tuple(sk_id, false, 0, 1, &mut buffer);
    assert_eq!(buffer[0], ObjectId::from(10));

    // Check Delta
    db.read_tuple(sk_id, true, 0, 1, &mut buffer);
    assert_eq!(buffer[0], ObjectId::from(20));

    Ok(())
}

/// # Objective
/// Deeply verify that indexing returns the exact correct memory offsets for specific values.
///
/// # Input
/// - Facts: [1, 10], [5, 50], [1, 99]
///
/// # Expected Output
/// - Offsets for value '1' should point exactly to [1, 10] and [1, 99].
/// - `fetch_tuple` using these offsets must return the expected secondary values (10 and 99).
#[test]
fn test_index_integrity() -> TestResult {
    let mut db = Database::new();
    let sk_id = AtomSkeletonId::from(1);

    db.insert_stable_fact(sk_id, &[ObjectId::from(1), ObjectId::from(10)]);
    db.insert_stable_fact(sk_id, &[ObjectId::from(5), ObjectId::from(50)]);
    db.insert_stable_fact(sk_id, &[ObjectId::from(1), ObjectId::from(99)]);

    let offsets = db
        .lookup_index(sk_id, false, ObjectId::from(1))
        .ok_or("Index failed")?;
    let mut buffer = [ObjectId::from(0); 2];

    // Verify first occurrence
    db.read_tuple(sk_id, false, offsets[0], 2, &mut buffer);
    assert_eq!(buffer[1], ObjectId::from(10));

    // Verify second occurrence
    db.read_tuple(sk_id, false, offsets[1], 2, &mut buffer);
    assert_eq!(buffer[1], ObjectId::from(99));

    Ok(())
}

/// # Objective
/// Ensure that different predicates (SkeletonIds) do not bleed into each other's storage.
///
/// # Input
/// - Predicate 1 contains [10].
/// - Predicate 2 contains [20].
///
/// # Expected Output
/// - `contains_stable` for Predicate 1 must NOT find [20].
#[test]
fn test_predicate_collision() {
    let mut db = Database::new();
    let id1 = AtomSkeletonId::from(1);
    let id2 = AtomSkeletonId::from(2);

    db.insert_stable_fact(id1, &[ObjectId::from(10)]);
    db.insert_stable_fact(id2, &[ObjectId::from(20)]);

    assert!(!db.contains_stable(id1, &[ObjectId::from(20)]));
    assert!(!db.contains_stable(id2, &[ObjectId::from(10)]));
}
