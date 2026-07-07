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

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::Relation;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId};
use rustc_hash::FxHashMap;

/// A two-tier relational database for Datalog facts.
///
/// It maintains two distinct sets of relations:
/// - **Stable**: Facts confirmed in previous iterations.
/// - **Delta**: New facts discovered during the current iteration.
///
/// This separation is essential for the Semi-Naive algorithm, ensuring that
/// each rule is only evaluated against at least one new fact from the Delta set.
///
/// # Memory Layout & Performance
///
/// * **Algorithmic Throughput**: This structure drops the standard library's SipHash layout in favor
///   of [`FxHashMap`] (via `rustc_hash`). Because [`AtomSkeletonId`] behaves fundamentally as a
///   lightweight integer key, `FxHash` bypasses cryptographic DOS protection to execute hash
///   lookups via elementary bit shifts, maximizing throughput during saturation.
/// * **Dual-Buffer Allocation**: The stable and delta collections isolate newly inferred facts,
///   preventing structural mutations or pointer invalidation inside the stable layers during a
///   fixed-point cycle.
#[derive(Default, Debug, Clone)]
pub struct Database {
    /// Facts that have been fully integrated into the knowledge base.
    stable: FxHashMap<AtomSkeletonId, Relation>,
    /// Buffer containing facts found in the latest saturation round.
    delta: FxHashMap<AtomSkeletonId, Relation>,
}

impl Database {
    /// Creates a new, empty Datalog database initialized with zero heap allocations.
    ///
    /// This constructor provides a clean state for bootstrapping the Semi-Naive
    /// evaluation engine, deferring internal storage map creation until the first
    /// insertion occurs.
    ///
    /// # Return Value
    ///
    /// Returns a fresh, default-initialized [`Self`] instance wrapping empty stable
    /// and delta relational structures.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ auxiliary memory footprint and execution overhead, as
    /// [`FxHashMap::default()`] does not allocate heap segments upfront.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a ground fact tuple into the stable relational storage.
    ///
    /// If the target relation corresponding to the given `skeleton_id` does not exist yet,
    /// it is lazily initialized using the arity inferred from the `args` slice. The fact
    /// is then passed down to the underlying structure which enforces set semantics.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The unique identifier matching the predicate definition schema.
    /// * `args` - A contiguous slice of constant [`ObjectId`] literals acting as the relation tuple.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the fact was genuinely new and successfully appended to the
    /// relation layout, and `false` if it triggered duplicate detection.
    ///
    /// # Complexity
    ///
    /// Amortized constant time average of $O(1)$ to compute the key hash and locate or insert
    /// the map entry, plus $O(A)$ where $A$ represents the arity (length) of the `args` slice
    /// to complete hash-based duplicate detection via `HashSet` and copy the constants into the [`Relation`].
    #[inline]
    pub fn insert_stable_fact(&mut self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        let arity = args.len();
        self.stable
            .entry(skeleton_id)
            .or_insert_with(|| Relation::new(arity))
            .insert(args)
    }

    /// Checks if a concrete tuple fact exists within the stable database layer.
    ///
    /// This method performs a read-only lookup inside the permanent knowledge base
    /// without scanning or affecting the transient delta buffer.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The identifier of the relation schema to query.
    /// * `args` - The constant tuple slice to search for.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the fact is validated inside the stable layer, and `false` otherwise.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ lookup to fetch the relation from the map, scaling
    /// linearly $O(A)$ with the target arity $A$ during the internal byte-slice comparison.
    #[inline]
    pub fn contains_stable(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.stable
            .get(&skeleton_id)
            .map_or(false, |rel| rel.contains(args))
    }

    /// Checks if a concrete tuple fact exists within the transient delta buffer.
    ///
    /// This method performs a read-only lookup inside the active generation's
    /// scratchpad, which isolates newly discovered facts before their promotion.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The identifier of the relation schema to query.
    /// * `args` - The constant tuple slice to search for.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the fact currently populates the delta layer, and `false` otherwise.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ lookup to fetch the relation from the delta map, scaling
    /// linearly $O(A)$ with the target arity $A$ during the internal byte-slice comparison.
    #[inline]
    pub fn contains_delta(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.delta
            .get(&skeleton_id)
            .map_or(false, |rel| rel.contains(args))
    }

    /// Evaluates if a concrete tuple fact is known anywhere across the multi-tier database storage.
    ///
    /// This method aggregates lookups from both the stable and delta buffers to determine
    /// global visibility of a fact within the current generation.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The unique predicate identifier schema to query.
    /// * `args` - The constant tuple slice to search for.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the tuple is present in either stable storage or the delta scratchpad.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ lookup, scaling linearly $O(A)$ with the target arity $A$
    /// due to internal byte comparisons. It benefits from short-circuiting evaluated from left to right.
    #[inline]
    pub fn has_fact(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.contains_stable(skeleton_id, args) || self.contains_delta(skeleton_id, args)
    }

    /// Retrieves a shared reference to a specific stable relation block if it exists.
    ///
    /// This lookup is typically used during rule evaluation cycles to inspect the
    /// current permanent knowledge base for a given predicate identifier.
    ///
    /// # Arguments
    ///
    /// * `skeleton_id` - The unique predicate identifier schema to look up.
    ///
    /// # Return Value
    ///
    /// Returns `Some(&Relation)` if initialized, or `None` if the database has not yet
    /// recorded any stable facts matching this schema.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ average footprint via `FxHash` lookup.
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
    pub fn stable_relations(&self) -> &FxHashMap<AtomSkeletonId, Relation> {
        &self.stable
    }

    /// Returns a shared reference to the internal map layout of delta buffer relations.
    ///
    /// This accessor is critical for the Semi-Naive evaluation engine to efficiently
    /// isolate and iterate over "newly discovered" facts that drive the next incremental
    /// rule instantiation round.
    ///
    /// # Return Value
    ///
    /// A shared reference to the underlying transient [`FxHashMap`] mapping
    /// [`AtomSkeletonId`] keys to their respective [`Relation`] objects.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ overhead with zero memory allocations or copy actions.
    #[inline]
    pub fn delta_relations(&self) -> &FxHashMap<AtomSkeletonId, Relation> {
        &self.delta
    }

    /// Aggregates the total number of unique atom tuples stored across all stable relations.
    ///
    /// This method iterates through every relation in the stable storage and aggregates
    /// their individual row counts. It serves as a vital metric for tracking the growth
    /// of the knowledge base.
    ///
    /// # Return Value
    ///
    /// The total count of unique ground facts (tuples) currently verified inside the
    /// stable knowledge base.
    ///
    /// # Usage
    ///
    /// This is a critical tracking metric for the saturation driver loop. By comparing
    /// this total count before and after an evaluation step, the engine can determine
    /// if new information was inferred or if a structural **fixed-point** (saturation)
    /// has been reached.
    ///
    /// # Complexity
    ///
    /// Linear time $O(R)$ where $R$ represents the total number of distinct predicate
    /// relation buckets registered in stable storage, executing a fast, allocation-free iteration.
    #[inline]
    pub fn total_facts_count(&self) -> usize {
        self.stable.values().map(|rel| rel.len()).sum()
    }

    /// Clears all relations from the stable storage layer.
    ///
    /// This removes all confirmed facts from the primary knowledge base but leaves
    /// the transient Delta buffer entirely intact.
    ///
    /// # Complexity
    ///
    /// Linear time $O(R)$ with respect to the number of registered relations $R$,
    /// triggering the structural deallocation or truncation of the underlying row buffers.
    #[inline]
    pub fn clear_stable(&mut self) {
        self.stable.clear();
    }

    /// Promotes all transient discoveries from the Delta buffer into permanent Stable storage.
    ///
    /// This method is called at the very end of each saturation iteration. It drains the
    /// Delta relations map to avoid heap reallocations, merging its structural content
    /// into the corresponding Stable relation buckets, clearing the Delta buffer for the next generation.
    ///
    /// # Complexity
    ///
    /// Time complexity scales with $O(T)$ where $T$ represents the absolute number of individual
    /// tuples currently residing inside the delta buffer layer, as each item is moved and re-inserted
    /// into the permanent stable mappings.
    pub fn commit_delta(&mut self) {
        for (sk_id, delta_rel) in self.delta.drain() {
            let arity = delta_rel.arity();
            let rel = self
                .stable
                .entry(sk_id)
                .or_insert_with(|| Relation::new(arity));

            if arity == 0 {
                // ARITY 0 CASE: Fast pathway for propositions.
                // If delta contains the truth token, insert it into stable.
                if !delta_rel.is_empty() {
                    rel.insert(&[]);
                }
            } else {
                // STANDARD CASE: Direct flat-buffer chunking.
                // Zero allocations, perfect L1/L2 cache locality, fully inlined.
                for tuple in delta_rel.data().chunks_exact(arity) {
                    rel.insert(tuple);
                }
            }
        }
    }

    /// Attempts to stage a new inferred discovery into the transient Delta buffer lane.
    ///
    /// A fact tuple is uniquely appended to the Delta tier if and only if it is completely absent
    /// from both the permanent Stable repository and the current generation's Delta scratchpad.
    /// This mutual exclusion is vital to prevent infinite looping and guarantee saturation termination.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique predicate skeleton token tracking the relation schema definition.
    /// * `args` - A contiguous slice of constant [`ObjectId`] literals acting as the relation row.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the fact is genuinely new to the entire database system and was successfully
    /// staged into the Delta buffer, and `false` if it triggered duplicate exclusion rules.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ average footprint to execute global exclusion lookups, scaling
    /// linearly $O(A)$ with the arity $A$ of the constant slice during binary row insertions.
    #[inline]
    pub fn insert_delta_fact(&mut self, sk_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        // 1. If the fact already exists anywhere within the storage layers, discard the insertion.
        if self.has_fact(sk_id, args) {
            return false;
        }

        // 2. Otherwise, allocate or append the new row into the delta scratchpad for the next round.
        let arity = args.len();
        self.delta
            .entry(sk_id)
            .or_insert_with(|| Relation::new(arity))
            .insert(args)
    }

    /// Returns a shared reference to a specific relation inside the transient Delta buffer lane.
    ///
    /// This is primarily used by the saturation engine to access the "pivot" relation
    /// during incremental Semi-Naive rule evaluations.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique predicate skeleton token tracking the target relation schema.
    ///
    /// # Return Value
    ///
    /// Returns `Some(&Relation)` if registered in the delta layer, or `None` if the database
    /// has not recorded any transient facts for this schema during the current round.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ average footprint via `FxHash` lookup.
    #[inline]
    pub fn get_delta_relation(&self, sk_id: AtomSkeletonId) -> Option<&Relation> {
        self.delta.get(&sk_id)
    }

    /// Transfers all stable facts directly into the Delta tier buffer lane.
    ///
    /// This method is utilized to bootstrap the Semi-Naive saturation process by treating
    /// the initial problem configuration state (e.g., the PDDL `init` block) as the original
    /// cohort of "newly discovered" facts.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ execution overhead. It leverages [`std::mem::swap`] to safely
    /// exchange internal pointer descriptors between hash map headers in place without
    /// re-allocating rows or losing structural capacities.
    #[inline]
    pub fn move_all_to_delta(&mut self) {
        // Swap both hash maps.
        // This is an atomic pointer operation: zero copies, zero allocations.
        std::mem::swap(&mut self.stable, &mut self.delta);
    }

    /// Inspects whether the transient Delta buffer has exhausted all fresh information.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the delta buffer lane is completely empty, signaling that
    /// a structural **fixed-point** saturation has successfully been achieved.
    ///
    /// # Complexity
    ///
    /// Constant time $O(1)$ evaluation execution.
    #[inline]
    pub fn is_delta_empty(&self) -> bool {
        self.delta.is_empty()
    }

    /// Completely flushes the database layout, clearing both Stable and Delta storages.
    ///
    /// This resets the database instance back to its initial pristine state. It is
    /// typically invoked when transitioning between distinct planning problems to
    /// prevent cross-contamination or data leakage.
    ///
    /// # Complexity
    ///
    /// Linear time $O(R_{stable} + R_{delta})$ with respect to the total number of
    /// registered relations across both storage layers, dropping or truncating all maps.
    #[inline]
    pub fn clear_all(&mut self) {
        self.stable.clear();
        self.delta.clear();
    }

    /// Retrieves structural memory layout metadata about a specific relation schema.
    ///
    /// This method targets the requested storage partition (`Stable` or `Delta`), but
    /// gracefully falls back to the alternate partition if the primary partition
    /// has not yet instantiated the relation bucket. This fallback ensures that
    /// query planners can always resolve the relational arity even immediately
    /// following an evaluation epoch swap.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique predicate skeleton identifier token to query.
    /// * `use_delta` - A flag directive; if `true`, scans the transient Delta scratchpad,
    ///   otherwise targets the permanent Stable layer.
    ///
    /// # Return Value
    ///
    /// Returns `Some((raw_buffer_length, arity))` tracking the absolute size of the internal
    /// flattened vector buffer along with the relation's fixed arity, or `None` if the
    /// relation has not yet been initialized in either layer.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ lookup footprint via dual-lookup `FxHash`.
    #[inline]
    pub fn get_layout(&self, sk_id: AtomSkeletonId, use_delta: bool) -> Option<(usize, usize)> {
        let primary = if use_delta {
            self.delta.get(&sk_id)
        } else {
            self.stable.get(&sk_id)
        };

        if let Some(r) = primary {
            Some((r.data().len(), r.arity()))
        } else {
            // FALLBACK STRATEGY: If the requested table is absent (e.g., stable right after a swap),
            // query the mirror table to extract at least the structural arity,
            // while simulating a data length of 0.
            let secondary = if use_delta {
                self.stable.get(&sk_id)
            } else {
                self.delta.get(&sk_id)
            };
            secondary.map(|r| (0, r.arity()))
        }
    }

    /// Reads a flat tuple from the raw relation buffer into a provided mutable output slice.
    ///
    /// This method extracts a specific slice of [`ObjectId`]s from the flattened contiguous vector
    /// layout of the relation, applying defensive bounds checking to prevent panics.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique predicate skeleton token targeting the relation schema.
    /// * `use_delta` - A flag directive; if `true`, reads from the transient Delta buffer,
    ///   otherwise targets the permanent Stable storage.
    /// * `start` - The linear memory offset indexing where the target tuple begins.
    /// * `arity` - The number of continuous elements (arity) to read.
    /// * `out` - The destination buffer slice, which must possess a capacity of at least `arity`.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ lookup to fetch the relation, plus linear time $O(A)$
    /// with respect to the arity $A$ to copy the data elements via sequential memory block copying.
    #[inline]
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
            // Safety boundary check to prevent out-of-bounds indexing or slicing panics.
            if start + arity <= data.len() {
                out[..arity].copy_from_slice(&data[start..start + arity]);
            }
        }
    }

    /// Performs an indexed lookup for facts sharing a specific leading constant argument.
    ///
    /// This method queries the structural index tied to the first position of the relation,
    /// significantly accelerating join operations by avoiding complete relation scans.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique predicate skeleton token targeting the relation schema.
    /// * `use_delta` - A flag directive; if `true`, scans the transient Delta index,
    ///   otherwise targets the permanent Stable index layout.
    /// * `first_arg` - The leading constant [`ObjectId`] key to filter on.
    ///
    /// # Return Value
    ///
    /// Returns `Some(&[usize])` containing a shared slice reference of raw memory offsets
    /// pointing to matching tuples, or `None` if no indexing exists or no matches are found.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ execution footprint. It fetches the relation and
    /// extracts the index map bucket slice with zero heap allocations or copy overhead.
    #[inline]
    pub fn lookup_index(
        &self,
        sk_id: AtomSkeletonId,
        use_delta: bool,
        first_arg: ObjectId,
    ) -> Option<&[usize]> {
        let rel = if use_delta {
            self.delta.get(&sk_id)
        } else {
            self.stable.get(&sk_id)
        };
        // Return a zero-cost slice view over the internal memory offsets vector.
        rel.and_then(|r| r.index_by_first_arg().get(&first_arg).map(|v| v.as_slice()))
    }

    /// Returns the total number of unique facts (tuples) associated with a given predicate.
    ///
    /// This method aggregates the row count from both the permanent Stable repository and the
    /// transient Delta buffer lane. It is designed as an ultra-fast metric, ideal for driving
    /// query optimization heuristics such as dynamic join ordering.
    ///
    /// # Arguments
    ///
    /// * `sk_id` - The unique identifier of the predicate schema to inspect.
    ///
    /// # Return Value
    ///
    /// The aggregated count of unique ground facts currently tracking under this schema.
    ///
    /// # Complexity
    ///
    /// Amortized constant time $O(1)$ average footprint, as it relies purely on the pre-calculated
    /// headers of the underlying relational storage structures.
    #[inline]
    pub fn get_relation_size(&self, sk_id: AtomSkeletonId) -> usize {
        let stable_size = self.stable.get(&sk_id).map(|r| r.len()).unwrap_or(0);
        let delta_size = self.delta.get(&sk_id).map(|r| r.len()).unwrap_or(0);
        stable_size + delta_size
    }
}

impl std::fmt::Display for Database {
    /// Formats the complete database structural state into a human-readable string representation.
    ///
    /// This implementation cascades down to the individual `Display` formatting routines
    /// provided by each [`Relation`] block, isolating the Stable repository from the Delta buffer.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== DATABASE STATE ===")?;

        // Section 1: Confirmed permanent Stable facts
        writeln!(f, "--- STABLE STORAGE ---")?;
        if self.stable.is_empty() {
            writeln!(f, "  (empty)")?;
        } else {
            for (sk_id, rel) in &self.stable {
                // Delegate to the relation's internal fmt implementation.
                writeln!(f, "  Relation #{} (arity {}): {}", sk_id, rel.arity(), rel)?;
            }
        }

        // Section 2: Newly discovered facts pending promotion to stable
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
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database;
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

    /// # Objective
    /// Verify that the custom `iter()` implementation correctly handles arity 0
    /// (propositions) without panicking and yields exactly one empty slice if true.
    ///
    /// # Input
    /// - Scenario A: Querying an uninitialized or empty relation of arity 0.
    /// - Scenario B: Inserting an empty fact vector `[]` (arity 0) into stable storage.
    ///
    /// # Expected Output
    /// - Scenario A: `iter().count()` returns `0`.
    /// - Scenario B: `iter()` yields exactly one item containing an empty slice `&[]`, then terminates.
    #[test]
    fn test_iter_arity_zero() {
        let mut db = Database::new();
        let sk_id = AtomSkeletonId::from(500);
        let empty_fact: Vec<ObjectId> = vec![];

        // Scenario A: Relation does not exist yet -> Should yield 0 items
        if let Some(rel) = db.get_relation(sk_id) {
            let count = rel.iter().count();
            assert_eq!(count, 0, "Empty relation should yield 0 items");
        }

        // Scenario B: Proposition is inserted (True) -> Should yield exactly 1 empty slice
        db.insert_stable_fact(sk_id, &empty_fact);
        let rel = db.get_relation(sk_id).expect("Relation must exist");

        let mut iter = rel.iter();
        let first_item = iter.next();

        assert!(first_item.is_some(), "Should yield exactly one item");
        assert_eq!(
            first_item.unwrap(),
            &[][..],
            "Yielded item must be an empty slice"
        );
        assert!(iter.next().is_none(), "Should not yield a second item");
    }

    /// # Objective
    /// Ensure that `commit_delta` correctly processes arity 0 facts via its dedicated
    /// fast pathway without dropping propositions or causing structural panics.
    ///
    /// # Input
    /// - Action: `insert_delta_fact` with an empty fact vector `[]` (arity 0), followed by `commit_delta()`.
    ///
    /// # Expected Output
    /// - Before commit: `contains_delta` is `true`.
    /// - After commit: `is_delta_empty()` is `true` and `contains_stable` becomes `true`.
    #[test]
    fn test_commit_delta_arity_zero() {
        let mut db = Database::new();
        let sk_id = AtomSkeletonId::from(777);
        let empty_fact: Vec<ObjectId> = vec![];

        // 1. Stage the proposition into delta
        db.insert_delta_fact(sk_id, &empty_fact);
        assert!(db.contains_delta(sk_id, &empty_fact));

        // 2. Commit to stable
        db.commit_delta();

        // 3. Verify successful promotion
        assert!(db.is_delta_empty());
        assert!(
            db.contains_stable(sk_id, &empty_fact),
            "Proposition must be successfully promoted to stable storage"
        );
    }

    /// # Objective
    /// Verify that `clear_all()` effectively clears all data from the maps,
    /// drops the internal Relation allocations, and resets the database state.
    ///
    /// # Input
    /// - Stable storage: `[10, 20]`
    /// - Delta storage: `[30, 40]`
    /// - Action: Call `clear_all()`.
    ///
    /// # Expected Output
    /// - `total_facts_count` is `0`.
    /// - `is_delta_empty()` is `true`.
    /// - Lookups for the cleared relations return `None`.
    #[test]
    fn test_clear_all_standard() {
        let mut db = Database::new();
        let sk_id = AtomSkeletonId::from(42);

        db.insert_stable_fact(sk_id, &[ObjectId::from(10), ObjectId::from(20)]);
        db.insert_delta_fact(sk_id, &[ObjectId::from(30), ObjectId::from(40)]);

        // Clear everything using the standard implementation
        db.clear_all();

        // Verify that the database is completely empty and structures are dropped
        assert_eq!(db.total_facts_count(), 0);
        assert!(db.is_delta_empty());
        assert!(
            db.get_relation(sk_id).is_none(),
            "Relation should be fully removed from stable"
        );
        assert!(
            db.get_delta_relation(sk_id).is_none(),
            "Relation should be fully removed from delta"
        );
    }
}
