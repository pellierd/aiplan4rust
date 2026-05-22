//! Preference Expressions Builder
//!
//! This module provides the implementation for preference-related expressions
//! in the LIR (Low-level Intermediate Representation). It handles the creation
//! of preference definitions and violation checks.
//!
//! All nodes created here are automatically interned to ensure that identical
//! preferences share the same [`ExprId`], enabling efficient $O(1)$ structural
//! equality checks during the planning process.

use crate::aiplan4rust::lang::PreferenceSymbolId;
use crate::aiplan4rust::lir::expr::ExprBuilder;
use crate::aiplan4rust::lir::expr::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Creates a leaf node representing a preference name.
    ///
    /// This is the foundation for all preference-related operations. By interning
    /// the name separately, the builder ensures that multiple references to the
    /// same preference share the same memory address.
    ///
    /// # Arguments
    ///
    /// * `id` - Any type convertible into a [`PreferenceSymbolId`] (e.g., usize, string-wrapper).
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for the interned preference name node.
    pub fn pref_name<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprEntryKind::PrefName(id.into()), &[])
    }

    /// Creates a `Preference` node, linking a name to a logical body.
    ///
    /// Represents the PDDL-style construction: `(preference <name> <body>)`.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier for the preference.
    /// * `body` - The [`ExprId`] of the logical condition or expression being preferred.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for the binary Preference node.
    pub fn preference<I: Into<PreferenceSymbolId>>(&mut self, id: I, body: ExprId) -> ExprId {
        let name_id = self.pref_name(id);
        self.intern(ExprEntryKind::Preference, &[name_id, body])
    }

    /// Creates an `IsViolated` check node for a specific preference.
    ///
    /// Represents the evaluation: `(is-violated <name>)`. This is typically
    /// used in numeric fluents or constraints to count violated preferences.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the preference to check.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The identifier for the unary IsViolated node.
    pub fn is_violated<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        let name_id = self.pref_name(id);
        self.intern(ExprEntryKind::IsViolated, &[name_id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::PreferenceSymbolId;
    use crate::aiplan4rust::lir::expr::ExprStore;

    /// Test: (preference P1 body)
    /// Verifies that the preference node is correctly constructed with two children:
    /// the interned name and the body.
    #[test]
    fn test_preference_construction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let body = builder.number(1.0);
        let pref_id = builder.preference(10, body);

        let node = builder.get(pref_id).unwrap();
        assert_eq!(node.kind(), &ExprEntryKind::Preference);
        assert_eq!(node.children().len(), 2);

        // Verify the first child is the PrefName node
        let name_node = builder.get(node.children()[0]).unwrap();
        assert!(
            matches!(name_node.kind(), ExprEntryKind::PrefName(id) if id == &PreferenceSymbolId::from(10))
        );
    }

    /// Test: Hash-Consing of Preference Names
    /// Ensures that calling pref_name multiple times for the same ID returns the same ExprId.
    #[test]
    fn test_preference_name_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let name1 = builder.pref_name(42);
        let name2 = builder.pref_name(42);

        assert_eq!(
            name1, name2,
            "Identical preference names must share the same ExprId"
        );
    }

    /// Test: Structural Equality between Preference and Violation
    /// Verifies that both a preference definition and its violation check
    /// share the exact same interned name node.
    #[test]
    fn test_preference_violation_sharing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let pref_symbol = 5;
        let body = builder.number(0.0);

        let pref_id = builder.preference(pref_symbol, body);
        let viol_id = builder.is_violated(pref_symbol);

        let pref_node = builder.get(pref_id).unwrap();
        let viol_node = builder.get(viol_id).unwrap();

        // Both nodes must have the same first child (the PrefName node)
        assert_eq!(
            pref_node.children()[0],
            viol_node.children()[0],
            "Preference and IsViolated must share the same interned name node"
        );
    }

    /// Test: (is-violated P1) construction
    #[test]
    fn test_is_violated_logic() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let viol_id = builder.is_violated(1);
        let node = builder.get(viol_id).unwrap();

        assert_eq!(node.kind(), &ExprEntryKind::IsViolated);
        assert_eq!(node.children().len(), 1, "IsViolated is a unary operator");
    }

    /// Test: Identity check with shared body
    /// Verifies that two different preferences can share the exact same
    /// body node without collision, while remaining distinct expressions.
    #[test]
    fn test_shared_body_distinct_preferences() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let shared_body = builder.number(42.0);

        let pref_a = builder.preference(1, shared_body);
        let pref_b = builder.preference(2, shared_body);

        assert_ne!(
            pref_a, pref_b,
            "Different names must result in different Preference IDs"
        );

        let node_a = builder.get(pref_a).unwrap();
        let node_b = builder.get(pref_b).unwrap();

        assert_eq!(
            node_a.children()[1],
            node_b.children()[1],
            "The body node must be structurally shared"
        );
    }

    /// Test: Deep Hash-Consing for IsViolated
    /// Ensures that an IsViolated node created independently is unified with one
    /// created after a Preference definition.
    #[test]
    fn test_is_violated_unification() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let id = 100;
        let v1 = builder.is_violated(id);
        let v2 = builder.is_violated(id);

        assert_eq!(
            v1, v2,
            "Redundant IsViolated checks must be unified to the same ExprId"
        );
    }
}
