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
#[path = "tests/database_tests.rs"]
mod database_tests;
