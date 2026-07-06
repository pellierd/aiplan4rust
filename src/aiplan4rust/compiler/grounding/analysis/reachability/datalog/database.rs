//! Relational storage for a Semi-Naive Datalog evaluation engine.
//!
//! This module provides the [`Database`] structure, which serves as the central
//! fact repository for the reachability analysis. It implements a dual-buffer
//! strategy (Stable vs. Delta) to support incremental inference without
//! redundant computations.
//!
//! The storage is optimized for:
//! - Fast fact insertion with duplicate detection.
//! - Efficient joins via first-argument indexing.
//! - Minimal memory overhead using raw tuple buffers.

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::relation::Relation;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId};
use std::collections::HashMap;

/// A two-tier relational database for Datalog facts.
///
/// It maintains two distinct sets of relations:
/// - **Stable**: Facts confirmed in previous iterations.
/// - **Delta**: New facts discovered during the current iteration.
///
/// This separation is essential for the Semi-Naive algorithm, ensuring that
/// each rule is only evaluated against at least one new fact from the Delta set.
#[derive(Default, Debug, Clone)]
pub struct Database {
    /// Facts that have been fully integrated into the knowledge base.
    stable: HashMap<AtomSkeletonId, Relation>,
    /// Buffer containing facts found in the latest saturation round.
    delta: HashMap<AtomSkeletonId, Relation>,
}

impl Database {
    /// Creates a new, empty Datalog database.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a fact into the stable storage.
    ///
    /// # Arguments
    /// * `skeleton_id` - The unique identifier for the predicate.
    /// * `args` - A slice of [`ObjectId`] representing the tuple of constants.
    ///
    /// # Returns
    /// * `true` if the fact was successfully inserted (i.e., it was not already present).
    /// * `false` if the fact was a duplicate.
    pub fn insert_stable_fact(&mut self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        let arity = args.len();
        self.stable
            .entry(skeleton_id)
            .or_insert_with(|| Relation::new(arity))
            .insert(args)
    }

    /// Checks if a fact exists within the stable storage.
    ///
    /// # Arguments
    /// * `skeleton_id` - The identifier of the relation to query.
    /// * `args` - The tuple to search for.
    ///
    /// # Returns
    /// `true` if the fact is found in the stable set.
    pub fn contains_stable(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.stable
            .get(&skeleton_id)
            .map_or(false, |rel| rel.contains(args))
    }

    /// Checks if a fact exists within the delta buffer.
    ///
    /// # Returns
    /// `true` if the fact is currently in the delta set.
    pub fn contains_delta(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.delta
            .get(&skeleton_id)
            .map_or(false, |rel| rel.contains(args))
    }

    /// Checks if a fact exists in either the stable or delta storage.
    ///
    /// # Returns
    /// `true` if the fact is known to the database.
    pub fn has_fact(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.contains_stable(skeleton_id, args) || self.contains_delta(skeleton_id, args)
    }

    /// Calculates the total number of unique facts across all stable relations.
    ///
    /// This is typically used to monitor the growth of the knowledge base.
    #[inline]
    pub fn get_relation(&self, skeleton_id: AtomSkeletonId) -> Option<&Relation> {
        self.stable.get(&skeleton_id)
    }

    /// Returns a reference to the internal map of stable relations.
    ///
    /// This accessor provides direct access to the entire set of confirmed facts,
    /// organized by their predicate identifiers.
    ///
    /// # Returns
    /// A reference to the [`HashMap`] containing all stable [`Relation`] objects.
    ///
    /// # Usage
    /// This is primarily used by the saturation algorithm to iterate over the
    /// current knowledge base and evaluate rules during the fixed-point calculation.
    #[inline]
    pub fn stable_relations(&self) -> &HashMap<AtomSkeletonId, Relation> {
        &self.stable
    }

    /// Returns a reference to the internal map of delta relations.
    ///
    /// This is crucial for the Semi-Naive algorithm to identify
    /// the "newly discovered" facts that trigger rules.
    #[inline]
    pub fn delta_relations(&self) -> &HashMap<AtomSkeletonId, Relation> {
        &self.delta
    }

    /// Calculates the total number of unique facts across all stable relations.
    ///
    /// This method iterates through every relation in the stable storage and
    /// aggregates their individual tuple counts.
    ///
    /// # Returns
    /// The total count of unique ground facts (tuples) currently stored in the
    /// stable knowledge base.
    ///
    /// # Usage
    /// This is a critical metric for the [`DatalogEngine`]. By comparing this count
    /// before and after a saturation step, the engine can determine if new
    /// information was discovered or if a **fixed-point** (saturation) has
    /// been reached.
    pub fn total_facts_count(&self) -> usize {
        self.stable.values().map(|rel| rel.len()).sum()
    }

    /// Clears all facts from the stable storage.
    ///
    /// This removes all confirmed relations but leaves the Delta buffer intact.
    pub fn clear_stable(&mut self) {
        self.stable.clear();
    }

    /// Promotes all facts from the Delta buffer to Stable storage.
    ///
    /// This method is called at the end of each saturation step. It drains the
    /// Delta set and merges its content into the Stable relations, clearing
    /// the Delta buffer for the next round.
    pub fn commit_delta(&mut self) {
        for (sk_id, delta_rel) in self.delta.drain() {
            let arity = delta_rel.arity();
            let rel = self
                .stable
                .entry(sk_id)
                .or_insert_with(|| Relation::new(arity));

            if arity == 0 {
                // CAS ARITÉ 0 : Si le delta n'est pas vide, la proposition est vraie.
                // On l'insère dans le stable via un tuple vide.
                if !delta_rel.is_empty() {
                    rel.insert(&[]);
                }
            } else {
                // CAS NORMAL : On itère sur les tuples de taille > 0.
                for tuple in delta_rel.iter() {
                    rel.insert(tuple);
                }
            }
        }
    }

    /// Attempts to insert a new discovery into the Delta buffer.
    ///
    /// A fact is only added to Delta if it is not already present in
    /// the Stable storage or the current Delta buffer.
    ///
    /// # Returns
    /// `true` if the fact is genuinely new and was added to the Delta.
    pub fn insert_delta_fact(&mut self, sk_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        // 1. Si on l'a déjà (n'importe où), on ne fait rien
        if self.has_fact(sk_id, args) {
            return false;
        }

        // 2. Sinon, on l'ajoute au delta pour le tour suivant
        let arity = args.len();
        self.delta
            .entry(sk_id)
            .or_insert_with(|| Relation::new(arity))
            .insert(args)
    }

    /// Returns a reference to a specific relation in the Delta buffer.
    ///
    /// This is primarily used to access the "pivot" relation during
    /// incremental evaluation.
    #[inline]
    pub fn get_delta_relation(&self, sk_id: AtomSkeletonId) -> Option<&Relation> {
        self.delta.get(&sk_id)
    }

    /// Transfers all stable facts to the Delta buffer.
    ///
    /// This is used to bootstrap the saturation process by treating the initial
    /// state (PDDL `init`) as the first set of "newly discovered" facts.
    pub fn move_all_to_delta(&mut self) {
        // On échange les maps pour que relations devienne delta
        self.delta = std::mem::take(&mut self.stable);
    }

    /// Returns `true` if the Delta buffer is empty.
    ///
    /// When this returns `true` after a saturation step, the fixed-point
    /// has been reached.
    #[inline]
    pub fn is_delta_empty(&self) -> bool {
        self.delta.is_empty()
    }

    /// Completely wipes the database, clearing both Stable and Delta storages.
    ///
    /// This resets the database to its initial empty state. It is typically
    /// called when transitioning between different planning problems to
    /// ensure no data leakage occurs.
    pub fn clear_all(&mut self) {
        self.stable.clear();
        self.delta.clear();
    }

    /// Retrieves structural information about a specific relation.
    ///
    /// # Returns
    /// An `Option` containing a tuple of `(raw_buffer_length, arity)`.
    pub fn get_layout(&self, sk_id: AtomSkeletonId, use_delta: bool) -> Option<(usize, usize)> {
        let rel = if use_delta {
            self.delta.get(&sk_id)
        } else {
            self.stable.get(&sk_id)
        };
        rel.map(|r| (r.data().len(), r.arity()))
    }

    /// Reads a tuple from the raw storage into a provided output buffer.
    ///
    /// # Arguments
    /// * `start` - The memory offset where the tuple begins.
    /// * `arity` - The number of elements to read.
    /// * `out` - The destination buffer (must be at least `arity` long).
    pub fn read_tuple(
        &self,
        sk_id: AtomSkeletonId,
        use_delta: bool,
        start: usize,
        arity: usize,
        out: &mut [ObjectId],
    ) {
        let rel_opt = if use_delta {
            self.delta.get(&sk_id)
        } else {
            self.stable.get(&sk_id)
        };
        if let Some(rel) = rel_opt {
            let data = rel.data();
            // Vérification de sécurité pour éviter le out-of-bounds
            if start + arity <= data.len() {
                out[..arity].copy_from_slice(&data[start..start + arity]);
            }
        }
    }

    /// Performs an indexed lookup for facts starting with a specific [`ObjectId`].
    ///
    /// # Returns
    /// A vector of memory offsets where matching tuples can be found.
    pub fn lookup_index(
        &self,
        sk_id: AtomSkeletonId,
        use_delta: bool,
        first_arg: ObjectId,
    ) -> Option<Vec<usize>> {
        let rel = if use_delta {
            self.delta.get(&sk_id)
        } else {
            self.stable.get(&sk_id)
        };
        // On clone le petit vecteur d'offsets (pas les données des faits)
        rel.and_then(|r| r.index_by_first_arg().get(&first_arg).cloned())
    }

    /// Returns the total number of unique facts (tuples) associated with a given predicate.
    ///
    /// This method aggregates the count from both the `stable` (fixed) and `delta` (newly discovered)
    /// storage layers. It is a constant-time $O(1)$ operation, making it ideal for
    /// query optimization heuristics such as join ordering.
    ///
    /// # Arguments
    /// * `sk_id` - The unique identifier of the predicate (skeleton).
    ///
    /// # Complexity
    /// $O(1)$ since it relies on the pre-calculated length of the underlying storage.
    #[inline]
    pub fn get_relation_size(&self, sk_id: AtomSkeletonId) -> usize {
        let stable_size = self.stable.get(&sk_id).map(|r| r.len()).unwrap_or(0);
        let delta_size = self.delta.get(&sk_id).map(|r| r.len()).unwrap_or(0);
        stable_size + delta_size
    }
}

impl std::fmt::Display for Database {
    /// Formats the database state into a human-readable representation.
    ///
    /// This implementation leverages the `Display` implementation of [`Relation`]
    /// to provide a clean overview of both Stable and Delta storages.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== DATABASE STATE ===")?;

        // Section 1: Confirmed Facts
        writeln!(f, "--- STABLE STORAGE ---")?;
        if self.stable.is_empty() {
            writeln!(f, "  (empty)")?;
        } else {
            for (sk_id, rel) in &self.stable {
                // rel est maintenant affiché via sa propre méthode fmt
                writeln!(f, "  Relation #{} (arity {}): {}", sk_id, rel.arity(), rel)?;
            }
        }

        // Section 2: Newly discovered facts pending promotion
        writeln!(f, "\n--- DELTA BUFFER ---")?;
        if self.delta.is_empty() {
            writeln!(f, "  (empty)")?;
        } else {
            for (sk_id, rel) in &self.delta {
                writeln!(f, "  Relation #{} (arity {}): {}", sk_id, rel.arity(), rel)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::database::Database;
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
}
