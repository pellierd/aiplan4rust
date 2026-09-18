//! # Datalog Engine Queries and Extractions Module
//!
//! This module provides query methods and extraction logic for the `DatalogEngine`.
//! It acts as an interface layer over the internal Datalog database (`Database`) and rule store,
//! allowing efficient retrieval of grounded elements required for planning analysis.
//!
//! ## Core Responsibilities
//! - **Reachable Fluents Extraction**: Retrieves all established ground facts belonging to the fluent predicate segment (`get_reachable_fluents`).
//! - **Reachable Actions Extraction**: Extracts valid action instances supporting both zero-arity and multi-argument definitions (`get_reachable_actions`).
//! - **Auxiliary Facts Retrieval**: Queries intermediate or derived structural axioms, such as `When` and `Derived` pivots (`get_reachable_auxiliaries`).
//! - **Type Extensions Mapping**: Maps structural type segment relations back to domain-level type identifiers (`get_type_extensions`).
//! - **Effects and Rules Lookups**: Provides direct dense-array and rule indexing for actions and auxiliary rules (`get_effects_for_action`, `get_rule_for_action`, `get_rule_for_auxiliary`).

use crate::aiplan4rust::support::lang::{ActionDefId, AtomSkeletonId, TypeId};
use crate::analysis::reachability::datalog::core::tuple::TupleArgs;
use crate::analysis::reachability::datalog::core::{Atom, Cause, Rule, Tuple};
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
    /// Extracts all reachable fluent tuples from the Datalog database storage.
    ///
    /// # Return Value
    /// Returns a [`Vec`] containing all tuples belonging to the fluent predicates segment.
    pub fn get_reachable_fluents(&self) -> Vec<Tuple<AtomSkeletonId>> {
        let mut fluents = Vec::new();

        // Iterate over the relations in the DB (Datalog storage)
        for (&sk_id, rel) in self.db.stable_relations().iter() {
            // Keep only what belongs to the fluents (predicates)
            if self.is_fluent(sk_id) {
                // The sk_id is already our internal AtomSkeletonId
                let skeleton_id = AtomSkeletonId::from(sk_id);

                for tuple_data in rel.iter() {
                    // Create a tuple for each row of the relation
                    let args = TupleArgs::from_slice(tuple_data);
                    fluents.push(Tuple::new(skeleton_id, args));
                }
            }
        }
        fluents
    }

    /// Extracts all reachable action tuples from the Datalog database storage, handling
    /// both zero-arity and multi-argument actions correctly.
    ///
    /// # Return Value
    /// Returns a [`Vec`] containing all reachable action definition tuples.
    pub fn get_reachable_actions(&self) -> Vec<Tuple<ActionDefId>> {
        let mut actions = Vec::with_capacity(self.db.stable_relations().len());

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            if self.is_action(sk_id) {
                let action_def_id = self.atom_id_to_action_def_id(sk_id);

                if rel.arity() == 0 {
                    // For zero arity, if the relation is not empty,
                    // it means the action is true (only 1 possible instance).
                    if !rel.is_empty() {
                        actions.push(Tuple::new(action_def_id, TupleArgs::new()));
                    }
                } else {
                    // For arity > 0, iterate normally over the arguments
                    for tuple_data in rel.iter() {
                        let args = TupleArgs::from_slice(tuple_data);
                        actions.push(Tuple::new(action_def_id, args));
                    }
                }
            }
        }
        actions
    }

    /// Extracts all reachable auxiliary facts (such as When and Derived pivots) from the database storage.
    ///
    /// # Return Value
    /// Returns a [`Vec`] containing all auxiliary tuples.
    pub fn get_reachable_auxiliaries(&self) -> Vec<Tuple<AtomSkeletonId>> {
        let mut axioms = Vec::new();

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            // Target only the auxiliary segment (When, Derived pivots, etc.)
            if self.is_auxiliary(sk_id) {
                let skeleton_id = AtomSkeletonId::from(sk_id);

                for tuple_data in rel.iter() {
                    let args = TupleArgs::from_slice(tuple_data);
                    axioms.push(Tuple::new(skeleton_id, args));
                }
            }
        }
        axioms
    }

    /// Extracts all type extensions mapped from the type segment relations.
    ///
    /// # Return Value
    /// Returns a [`Vec`] containing all type extension tuples.
    pub fn get_type_extensions(&self) -> Vec<Tuple<TypeId>> {
        // Pre-allocate relative to the number of relations, just like for actions
        let mut types = Vec::with_capacity(self.db.stable_relations().len());

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            let id_val = sk_id;

            // 1. Use the segment method for types
            if self.is_type(id_val) {
                // 2. Inline arithmetic translation (O(1))
                // Subtract fluence_threshold to find the typing index
                let type_id = self.atom_id_to_type_id(sk_id);

                for tuple_data in rel.iter() {
                    // 3. Create the tuple (often unary for types)
                    let args = TupleArgs::from_slice(tuple_data);
                    types.push(Tuple::new(type_id, args));
                }
            }
        }
        types
    }

    /// Retrieves the effects (Add and Delete) and their causality associated with a specific action.
    ///
    /// # Arguments
    /// * `action_sk_id` - The skeleton identifier of the action atom produced by the Datalog engine.
    ///
    /// # Return Value
    /// Returns a slice of pairs containing the effect atom and its corresponding [`Cause`].
    /// Returns an empty slice if no effects are found for the given action.
    ///
    /// # Panics
    /// Panics in debug mode (`debug_assert!`) if the provided identifier does not point to a valid action.
    pub fn get_effects_for_action(&self, action_sk_id: AtomSkeletonId) -> &[(Atom, Cause)] {
        // 1. Verify that it is indeed an action (via the ID segmentation mechanism)
        debug_assert!(self.is_action(action_sk_id));

        // 2. Calculate the relative index to access the dense Vec
        // Ensure action_base_id correctly matches the first ID allocated to actions.
        let action_index = action_sk_id.as_usize() - self.action_base_id;

        // 3. Directly access our internal effects array without going through the encoder
        self.action_effects
            .get(action_index)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Retrieves the rule associated with a specific action definition identifier.
    ///
    /// # Arguments
    /// * `action_index` - The [`ActionDefId`] of the target action.
    ///
    /// # Return Value
    /// Returns a reference to the [`Rule`] that defines the given action wrapped in a [`Result`].
    ///
    /// # Errors
    /// Returns a [`DatalogError::ActionRuleNotFound`] if no rule is found for the specified action index.
    pub fn get_rule_for_action(&self, action_index: ActionDefId) -> Result<&Rule, DatalogError> {
        // The internal ID is calculated directly here using the action definition ID
        let target_sk_id = AtomSkeletonId::from(self.type_threshold + action_index.as_usize());

        self.rules
            .iter()
            .find(|r| r.head().symbol() == target_sk_id)
            .ok_or_else(|| DatalogError::action_rule_not_found(action_index))
    }

    /// Retrieves the rule associated with a specific auxiliary atom skeleton identifier.
    ///
    /// # Arguments
    /// * `sk_id` - The [`AtomSkeletonId`] of the auxiliary fact.
    ///
    /// # Return Value
    /// Returns a reference to the corresponding [`Rule`] wrapped in a [`Result`].
    ///
    /// # Errors
    /// Returns a [`DatalogError::AuxiliaryRuleNotFound`] if an auxiliary fact is found without a matching rule.
    ///
    /// # Panics
    /// Panics in debug mode if `sk_id` is not an auxiliary.
    pub fn get_rule_for_auxiliary(&self, sk_id: AtomSkeletonId) -> Result<&Rule, DatalogError> {
        debug_assert!(self.is_auxiliary(sk_id));

        self.rules
            .iter()
            .find(|r| r.head().symbol() == sk_id)
            .ok_or_else(|| DatalogError::auxiliary_rule_not_found(sk_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::core::atom::AtomArgs;
    use rustc_hash::FxHashMap;

    /// Helper to bootstrap a mocked DatalogEngine for unit tests.
    ///
    /// # Returns
    /// A preconfigured `DatalogEngine` instance with default thresholds and empty stores.
    fn create_mock_engine<'a>() -> DatalogEngine<'a> {
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
            type_segment_start: 3,
            type_threshold: 5,
            action_base_id: 5,
            action_threshold: 8,
            builtin_threshold: 10,
            union_cache: FxHashMap::default(),
            base_aux_id: 8,
            next_aux_id: 8,
            aux_defs: Vec::new(),
            cache: FxHashMap::default(),
            action_effects: Vec::new(),
            action_anchor: None,
            negation_offset: 0,
            type_to_skeleton: Vec::new(),
        }
    }

    /// # Test: Get Reachable Fluents
    ///
    /// ## Objective
    /// Validates that `get_reachable_fluents` correctly extracts stable facts falling within the fluent segment.
    ///
    /// ## Input
    /// - A database containing a stable fact mapped to `fluent_id` (0) with object ID `42`.
    ///
    /// ## Expected Output
    /// - A vector containing exactly one tuple with symbol `fluent_id` and the associated argument `42`.
    #[test]
    fn test_get_reachable_fluents() {
        let mut engine = create_mock_engine();
        let fluent_id = AtomSkeletonId::from(0);
        engine.db.insert_stable_fact(
            fluent_id,
            &[crate::aiplan4rust::support::lang::ObjectId::from(42)],
        );

        let fluents = engine.get_reachable_fluents();
        assert_eq!(
            fluents.len(),
            1,
            "Should retrieve exactly one reachable fluent"
        );
        assert_eq!(fluents[0].symbol(), fluent_id);
    }

    /// # Test: Get Type Extensions
    ///
    /// ## Objective
    /// Validates that `get_type_extensions` correctly extracts and maps structural type IDs from type segment relations.
    ///
    /// ## Input
    /// - A database containing a stable fact at skeleton ID `3` (type segment) with object ID `100`.
    ///
    /// ## Expected Output
    /// - A vector containing one type extension tuple mapped to `TypeId(0)` with object ID `100`.
    #[test]
    fn test_get_type_extensions() {
        let mut engine = create_mock_engine();
        let type_sk_id = AtomSkeletonId::from(3); // within [3..5[
        engine.db.insert_stable_fact(
            type_sk_id,
            &[crate::aiplan4rust::support::lang::ObjectId::from(100)],
        );

        let types = engine.get_type_extensions();
        assert_eq!(types.len(), 1, "Should retrieve one type extension");
        assert_eq!(types[0].symbol(), TypeId::from(0)); // 3 - 3 = 0
    }

    /// # Test: Get Reachable Actions
    ///
    /// ## Objective
    /// Validates that `get_reachable_actions` properly extracts both zero-arity actions and multi-argument actions.
    ///
    /// ## Input
    /// - An arity 0 action registered at skeleton ID `5`.
    /// - A multi-argument action registered at skeleton ID `6` with object ID `7`.
    ///
    /// ## Expected Output
    /// - A vector containing exactly two reachable action tuples.
    #[test]
    fn test_get_reachable_actions() {
        let mut engine = create_mock_engine();

        // Arity 0 action
        let action_sk_zero = AtomSkeletonId::from(5);
        engine.db.insert_stable_fact(action_sk_zero, &[]);

        // Arity > 0 action
        let action_sk_nonzero = AtomSkeletonId::from(6);
        engine.db.insert_stable_fact(
            action_sk_nonzero,
            &[crate::aiplan4rust::support::lang::ObjectId::from(7)],
        );

        let actions = engine.get_reachable_actions();
        assert_eq!(
            actions.len(),
            2,
            "Should retrieve both zero-arity and multi-argument actions"
        );
    }

    /// # Test: Get Reachable Auxiliaries
    ///
    /// ## Objective
    /// Validates that `get_reachable_auxiliaries` extracts facts belonging to the auxiliary segment.
    ///
    /// ## Input
    /// - An auxiliary stable fact registered at skeleton ID `8` with object ID `10`.
    ///
    /// ## Expected Output
    /// - A vector containing one auxiliary tuple matching skeleton ID `8`.
    #[test]
    fn test_get_reachable_auxiliaries() {
        let mut engine = create_mock_engine();
        let aux_sk_id = AtomSkeletonId::from(8);
        engine.db.insert_stable_fact(
            aux_sk_id,
            &[crate::aiplan4rust::support::lang::ObjectId::from(10)],
        );

        let auxiliaries = engine.get_reachable_auxiliaries();
        assert_eq!(auxiliaries.len(), 1, "Should retrieve one auxiliary fact");
        assert_eq!(auxiliaries[0].symbol(), aux_sk_id);
    }

    /// # Test: Get Effects and Rules
    ///
    /// ## Objective
    /// Validates the lookup functions for action effects, action rules (using `ActionDefId`), and auxiliary rules.
    ///
    /// ## Input
    /// - Action effects vector configured with an entry for action index 0.
    /// - A rule mapped to an action head.
    /// - A rule mapped to an auxiliary head.
    ///
    /// ## Expected Output
    /// - An empty slice of effects for the action.
    /// - Successful Result-wrapped rule retrievals matching the expected head symbols for both action and auxiliary.
    #[test]
    fn test_get_effects_and_rules() -> Result<(), DatalogError> {
        let mut engine = create_mock_engine();
        let action_sk_zero = AtomSkeletonId::from(5);
        let aux_sk_id = AtomSkeletonId::from(8);

        // 1. Test effects for action
        engine.action_effects.push(vec![]);
        let effects = engine.get_effects_for_action(action_sk_zero);
        assert!(effects.is_empty(), "Effects slice should be empty");

        // 2. Test rule for action lookup (using ActionDefId and handling Result)
        let head_atom = Atom::nary(action_sk_zero, AtomArgs::new());
        let rule_for_action = Rule::new(head_atom, vec![]);
        engine.rules.push(rule_for_action);

        let action_def_id = ActionDefId::from(0); // Corresponds to action_sk_zero (5 - type_threshold[5] = 0)
        let retrieved_rule = engine.get_rule_for_action(action_def_id)?;
        assert_eq!(retrieved_rule.head().symbol(), action_sk_zero);

        // 3. Test rule for auxiliary lookup (handling Result)
        let aux_atom = Atom::nary(aux_sk_id, AtomArgs::new());
        let aux_rule = Rule::new(aux_atom, vec![]);
        engine.rules.push(aux_rule);
        let retrieved_aux_rule = engine.get_rule_for_auxiliary(aux_sk_id)?;
        assert_eq!(retrieved_aux_rule.head().symbol(), aux_sk_id);

        Ok(())
    }

    /// # Test: Get Reachable Actions Edge Cases
    ///
    /// ## Objective
    /// Validates edge cases for action retrieval, including empty zero-arity relations and multi-instance multi-argument actions.
    ///
    /// ## Input
    /// - An empty zero-arity action relation at skeleton ID `5`.
    /// - A multi-argument action at skeleton ID `6` with two distinct object instances (`1` and `2`).
    ///
    /// ## Expected Output
    /// - Empty relation is ignored, and both instances of the multi-argument action are correctly retrieved (total length: 2).
    #[test]
    fn test_get_reachable_actions_edge_cases() {
        let mut engine = create_mock_engine();

        // Case 1: Empty zero-arity action (should not be retrieved)
        let _action_sk_empty = AtomSkeletonId::from(5);
        // Relation is left empty intentionally

        // Case 2: Multi-argument action with multiple instances
        let action_sk_multi = AtomSkeletonId::from(6);
        engine.db.insert_stable_fact(
            action_sk_multi,
            &[crate::aiplan4rust::support::lang::ObjectId::from(1)],
        );
        engine.db.insert_stable_fact(
            action_sk_multi,
            &[crate::aiplan4rust::support::lang::ObjectId::from(2)],
        );

        let actions = engine.get_reachable_actions();
        assert_eq!(
            actions.len(),
            2,
            "Should retrieve both instances of the multi-argument action"
        );
    }
}
