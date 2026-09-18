//! # Datalog Reachability Segments and ID Mapping
//!
//! This module manages the structural memory layout and interval boundaries for
//! `AtomSkeletonId` within the Datalog reachability engine.
//!
//! ## Overview
//! The Datalog engine partitions its internal atom space into contiguous, non-overlapping
//! segments to efficiently process grounding, types, actions, and auxiliary facts:
//!
//! 1. **Positive Fluents**: Ranging from index `0` up to `fluence_threshold`.
//! 2. **Negated Fluents**: Ranging from `fluence_threshold` up to `type_segment_start`.
//!    Provides a mirror zone for negative literals.
//! 3. **Types**: Ranging from `type_segment_start` up to `type_threshold`.
//!    Mapped directly to structural `TypeId` instances.
//! 4. **Actions**: Ranging from `type_threshold` up to `action_threshold`.
//!    Mapped directly to structural `ActionDefId` instances.
//! 5. **Auxiliaries**: Ranging from `action_threshold` up to `Atom::BUILTIN_ZONE_START`.
//! 6. **Built-ins**: Anything at or beyond `Atom::BUILTIN_ZONE_START`.
//!
//! ## Key Features
//! - Strict boundary checking (`is_fluent`, `is_type`, `is_action`, `is_auxiliary`, `is_builtin`).
//! - Bidirectional conversions between internal skeleton IDs and external identifiers (`TypeId`, `ActionDefId`).
//! - Symmetry management for positive and negated atoms (`negate_id`, `pos_id_from_negated`).

use crate::aiplan4rust::support::lang::{ActionDefId, AtomSkeletonId, TypeId};
use crate::analysis::reachability::datalog::core::Atom;
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
    /// Checks whether the given atom skeleton ID represents a fluent.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID is strictly before the type segment start (covering both positive and optional negative blocks), `false` otherwise.
    #[inline]
    pub fn is_fluent(&self, id: AtomSkeletonId) -> bool {
        // Anything before the start of types is a fluent
        // (this includes both the positive block and the optional negative block)
        id.as_usize() < self.type_segment_start
    }

    /// Checks whether the given atom skeleton ID represents a negated fluent.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID falls within the negated fluent range, `false` otherwise.
    #[inline]
    pub fn is_negated_fluent(&self, id: AtomSkeletonId) -> bool {
        let val = id.as_usize();
        val >= self.fluence_threshold && val < self.type_segment_start
    }

    /// Checks whether the given atom skeleton ID represents a type predicate.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID is located within the type segment boundaries, `false` otherwise.
    #[inline]
    pub fn is_type(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // The type segment is sandwiched between its own boundary and the start of actions
        p >= self.type_segment_start && p < self.type_threshold
    }

    /// Converts an atom skeleton ID belonging to the type segment into a `TypeId`.
    ///
    /// # Arguments
    /// * `sk_id` - The `AtomSkeletonId` corresponding to a type.
    ///
    /// # Returns
    /// The corresponding structural `TypeId`.
    #[inline]
    pub fn atom_id_to_type_id(&self, sk_id: AtomSkeletonId) -> TypeId {
        let id_val = sk_id.as_usize();
        debug_assert!(self.is_type(sk_id));
        // The subtraction offset is now dynamic
        TypeId::from(id_val - self.type_segment_start)
    }

    /// Computes the negated counterpart of a positive fluent atom skeleton ID.
    ///
    /// # Arguments
    /// * `id` - The positive `AtomSkeletonId`.
    ///
    /// # Returns
    /// The negated `AtomSkeletonId` corresponding to the input.
    #[inline]
    pub fn negate_id(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // We assume that id is a positive fluent < fluence_threshold
        debug_assert!(val < self.fluence_threshold);
        AtomSkeletonId::from(val + self.fluence_threshold)
    }

    /// Retrieves the original positive fluent atom skeleton ID from its negated counterpart.
    ///
    /// # Arguments
    /// * `id` - The negated `AtomSkeletonId`.
    ///
    /// # Returns
    /// The underlying positive `AtomSkeletonId`.
    #[inline]
    pub fn pos_id_from_negated(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // We assume that id is a negative fluent [N..2N[
        debug_assert!(val >= self.fluence_threshold && val < self.type_segment_start);
        AtomSkeletonId::from(val - self.fluence_threshold)
    }

    /// Checks whether an atom skeleton ID belongs to the Action segment.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID falls within the action boundaries, `false` otherwise.
    #[inline]
    pub fn is_action(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        p >= self.type_threshold && p < self.action_threshold
    }

    /// Converts an action-segment atom skeleton ID into an `ActionDefId`.
    ///
    /// # Arguments
    /// * `sk_id` - The `AtomSkeletonId` corresponding to an action.
    ///
    /// # Returns
    /// The corresponding `ActionDefId`.
    #[inline]
    pub fn atom_id_to_action_def_id(&self, sk_id: AtomSkeletonId) -> ActionDefId {
        let id_val = sk_id.as_usize();
        // We don't touch anything here, the calculation is mathematically correct for the vector
        ActionDefId::from(id_val - self.type_threshold)
    }

    /// Converts a public `ActionDefId` into an internal `AtomSkeletonId`.
    ///
    /// # Arguments
    /// * `def_id` - The `ActionDefId` to convert.
    ///
    /// # Returns
    /// The mapped `AtomSkeletonId`.
    pub fn action_def_id_to_atom_id(&self, def_id: ActionDefId) -> AtomSkeletonId {
        AtomSkeletonId::from(def_id.as_usize() + self.action_base_id)
    }

    /// Converts an `ActionDefId` into the internal `AtomSkeletonId`
    /// utilized specifically by the Datalog engine.
    ///
    /// # Arguments
    /// * `action_id` - The `ActionDefId` to convert.
    ///
    /// # Returns
    /// The target `AtomSkeletonId`.
    pub fn action_id_to_skeleton(&self, action_id: ActionDefId) -> AtomSkeletonId {
        // We use the same calculation as atom_id_to_action_def_id but in reverse
        AtomSkeletonId::from(action_id.as_usize() + self.type_threshold)
    }

    /// Checks whether the given atom skeleton ID belongs to the built-in zone.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID is at or beyond the built-in zone threshold, `false` otherwise.
    #[inline]
    pub fn is_builtin(&self, id: AtomSkeletonId) -> bool {
        id.as_usize() >= Atom::BUILTIN_ZONE_START
    }

    /// Checks whether the given atom skeleton ID represents an auxiliary atom.
    ///
    /// # Arguments
    /// * `id` - The `AtomSkeletonId` to check.
    ///
    /// # Returns
    /// `true` if the ID lies between the end of actions and the built-ins zone, `false` otherwise.
    #[inline]
    pub fn is_auxiliary(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // An auxiliary is anything located between the end of actions
        // and the start of the zone reserved for built-ins (equality).
        p >= self.action_threshold && p < Atom::BUILTIN_ZONE_START
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
    use crate::aiplan4rust::support::lang::{ActionDefId, AtomSkeletonId, TypeId};
    use crate::analysis::inertia::table::InertiaTable;
    use crate::DatalogEngine;
    use rustc_hash::FxHashMap;

    /// Helper to create a pre-configured engine with custom segment boundaries for testing.
    fn create_segmented_engine<'a>() -> DatalogEngine<'a> {
        let problem_ref = Box::leak(Box::new(LiftedProblem::default()));
        let registry_ref = Box::leak(Box::new(ValueRegistry::default()));
        let table_ref = Box::leak(Box::new(InertiaTable::empty()));
        let neg_ref = Box::leak(Box::new(Vec::new()));

        DatalogEngine {
            problem: problem_ref,
            value_registry: registry_ref,
            inertia_table: table_ref,
            negated_predicates: neg_ref,
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; crate::analysis::reachability::datalog::engine::MAX_VARS],
            trailing_indices: Vec::new(),
            discovered_facts: Vec::new(),
            head_buffer: Vec::new(),
            fluence_threshold: 2,
            type_segment_start: 4, // Negative/mirror zone from 2 to 4
            type_threshold: 7,     // 3 types (indices 4, 5, 6)
            action_threshold: 10,  // 3 actions (indices 7, 8, 9)
            action_base_id: 7,
            builtin_threshold: 0,
            union_cache: FxHashMap::default(),
            base_aux_id: 0,
            next_aux_id: 0,
            aux_defs: Vec::new(),
            cache: FxHashMap::default(),
            action_effects: Vec::new(),
            action_anchor: None,
            negation_offset: 0,
            type_to_skeleton: Vec::new(),
        }
    }

    /// # Test: Fluent and Negation Segment Boundaries
    ///
    /// ## Objective
    /// Validates strict interval segregation and symmetry logic for fluents and negated fluents.
    ///
    /// ## Input
    /// - An engine with `fluence_threshold = 2` and `type_segment_start = 4`.
    ///
    /// ## Expected Output
    /// - Proper identification of positive and negative fluents, and correct bidirectional ID negation mappings.
    #[test]
    fn test_fluent_and_negation_boundaries() {
        let engine = create_segmented_engine();

        assert!(
            engine.is_fluent(AtomSkeletonId::from(0)),
            "0 must be a positive fluent"
        );
        assert!(
            engine.is_fluent(AtomSkeletonId::from(1)),
            "1 must be a positive fluent"
        );
        assert!(
            !engine.is_negated_fluent(AtomSkeletonId::from(1)),
            "1 must not be a negated fluent"
        );

        assert!(
            engine.is_fluent(AtomSkeletonId::from(2)),
            "2 must be a fluent (negative)"
        );
        assert!(
            engine.is_negated_fluent(AtomSkeletonId::from(2)),
            "2 must be a negated fluent"
        );
        assert!(
            engine.is_negated_fluent(AtomSkeletonId::from(3)),
            "3 must be a negated fluent"
        );
        assert!(
            !engine.is_negated_fluent(AtomSkeletonId::from(4)),
            "4 is the type boundary, not a negated fluent"
        );

        // Test positive / negative symmetry
        let pos_id = AtomSkeletonId::from(1);
        let neg_id = engine.negate_id(pos_id);
        assert_eq!(neg_id, AtomSkeletonId::from(3));
        assert_eq!(engine.pos_id_from_negated(neg_id), pos_id);
    }

    /// # Test: Type Segment Boundaries and Conversions
    ///
    /// ## Objective
    /// Validates that structural type segment bounds are respected and correctly translated to `TypeId`.
    ///
    /// ## Input
    /// - An engine with type segment mapped between indices 4 and 7.
    ///
    /// ## Expected Output
    /// - Correct classification of type IDs and accurate mapping from `AtomSkeletonId` to `TypeId`.
    #[test]
    fn test_type_segment_conversions() {
        let engine = create_segmented_engine();

        assert!(
            !engine.is_type(AtomSkeletonId::from(3)),
            "3 is before the type segment"
        );
        assert!(
            engine.is_type(AtomSkeletonId::from(4)),
            "4 is the start of types"
        );
        assert!(
            engine.is_type(AtomSkeletonId::from(6)),
            "6 is within the types"
        );
        assert!(
            !engine.is_type(AtomSkeletonId::from(7)),
            "7 is already an action"
        );

        assert_eq!(
            engine.atom_id_to_type_id(AtomSkeletonId::from(4)),
            TypeId::from(0)
        );
        assert_eq!(
            engine.atom_id_to_type_id(AtomSkeletonId::from(6)),
            TypeId::from(2)
        );
    }

    /// # Test: Action Segment Boundaries and Conversions
    ///
    /// ## Objective
    /// Validates action segment ranges and bidirectional translations between `AtomSkeletonId` and `ActionDefId`.
    ///
    /// ## Input
    /// - An engine with action segment mapped between indices 7 and 10 (`action_base_id = 7`).
    ///
    /// ## Expected Output
    /// - Accurate detection of action boundaries and correct conversion indexes.
    #[test]
    fn test_action_segment_conversions() {
        let engine = create_segmented_engine();

        assert!(!engine.is_action(AtomSkeletonId::from(6)), "6 is a type");
        assert!(
            engine.is_action(AtomSkeletonId::from(7)),
            "7 is the start of actions"
        );
        assert!(
            engine.is_action(AtomSkeletonId::from(8)),
            "8 is an intermediate action"
        );
        assert!(
            engine.is_action(AtomSkeletonId::from(9)),
            "9 is the last action of the block"
        );
        assert!(
            !engine.is_action(AtomSkeletonId::from(10)),
            "10 is the auxiliary threshold"
        );

        assert_eq!(
            engine.atom_id_to_action_def_id(AtomSkeletonId::from(7)),
            ActionDefId::from(0)
        );
        assert_eq!(
            engine.atom_id_to_action_def_id(AtomSkeletonId::from(8)),
            ActionDefId::from(1)
        );
        assert_eq!(
            engine.action_def_id_to_atom_id(ActionDefId::from(1)),
            AtomSkeletonId::from(8)
        );
        assert_eq!(
            engine.action_id_to_skeleton(ActionDefId::from(2)),
            AtomSkeletonId::from(9)
        );
    }

    /// # Test: Auxiliary Segment Boundaries
    ///
    /// ## Objective
    /// Validates that auxiliary axioms and derived pivots correctly start past the action threshold.
    ///
    /// ## Input
    /// - An engine with action threshold at 10.
    ///
    /// ## Expected Output
    /// - Confirms index 9 is an action and index 10 marks the exact start of the auxiliary segment.
    #[test]
    fn test_auxiliary_segment_boundaries() {
        let engine = create_segmented_engine();

        assert!(
            !engine.is_auxiliary(AtomSkeletonId::from(9)),
            "9 is an action, not an auxiliary"
        );
        assert!(
            engine.is_auxiliary(AtomSkeletonId::from(10)),
            "10 is the exact start of auxiliaries"
        );
    }
}
