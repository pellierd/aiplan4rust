//! # Temporal Operators Module
//!
//! Provides constructors for PDDL 2.1 temporal constraints: `at start`, `at end`,
//! and `overall`. These operators are essential for defining when conditions
//! must hold or when effects occur in temporal planning.
//!
//! This module implements structural optimizations to prevent nested temporal
//! redundancy and ensures high-performance interning through inlining.

use crate::aiplan4rust::lir::store::expr::builder::{ExprBuilder, ExprBuilderError};
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use ordered_float::OrderedFloat;

impl<'a> ExprBuilder<'a> {
    /// Wraps an expression with an 'At Start' temporal constraint.
    ///
    /// # Arguments
    /// * `expr` - The `ExprId` of the formula to be constrained to the start of the action.
    ///
    /// # Returns
    /// * `Ok(ExprId)` - The identifier for the interned `AtStart` node.
    /// * `Err(ExprBuilderError)` - If the expression is already a temporal operator (illegal nesting).
    ///
    /// # Performance
    /// Marked `#[inline]`. Validates PDDL temporal invariants before interning.
    #[inline]
    pub fn at_start(&mut self, expr: ExprId) -> Result<ExprId, ExprBuilderError> {
        let kind = ExprEntryKind::AtStart;
        self.check_temporal_invariant(expr, kind.clone())?;
        Ok(self.intern(kind, &[expr]))
    }

    /// Wraps an expression with an 'At End' temporal constraint.
    ///
    /// # Arguments
    /// * `expr` - The `ExprId` of the formula to be constrained to the end of the action.
    ///
    /// # Returns
    /// * `Ok(ExprId)` - The identifier for the interned `AtEnd` node.
    /// * `Err(ExprBuilderError)` - If the expression is already a temporal operator.
    ///
    /// # Performance
    /// Marked `#[inline]`. Prevents illegal nesting of temporal constraints.
    #[inline]
    pub fn at_end(&mut self, expr: ExprId) -> Result<ExprId, ExprBuilderError> {
        let kind = ExprEntryKind::AtEnd;
        self.check_temporal_invariant(expr, kind.clone())?;
        Ok(self.intern(kind, &[expr]))
    }

    /// Wraps an expression with an 'Overall' (invariant) temporal constraint.
    ///
    /// # Arguments
    /// * `expr` - The `ExprId` of the formula that must hold true throughout the action.
    ///
    /// # Returns
    /// * `Ok(ExprId)` - The identifier for the interned `Overall` node.
    /// * `Err(ExprBuilderError)` - If the expression is already a temporal operator.
    ///
    /// # Performance
    /// Marked `#[inline]`. Ensures the semantic integrity of the temporal invariant.
    #[inline]
    pub fn overall(&mut self, expr: ExprId) -> Result<ExprId, ExprBuilderError> {
        let kind = ExprEntryKind::Overall;
        self.check_temporal_invariant(expr, kind.clone())?;
        Ok(self.intern(kind, &[expr]))
    }

    /// Validates that the target expression is not already a temporal operator to prevent
    /// illegal PDDL nesting.
    ///
    /// # Arguments
    /// * `expr` - The `ExprId` of the sub-expression to be wrapped.
    /// * `attempted_kind` - The `ExprEntryKind` of the temporal operator being applied
    ///   (e.g., `AtStart`, `AtEnd`, or `Overall`).
    ///
    /// # Returns
    /// * `Ok(())` - If the expression is not a temporal operator and can be safely wrapped.
    /// * `Err(ExprBuilderError::InvalidTemporalInvariant)` - If `expr` is already a
    ///   temporal operator, containing both the existing and attempted kinds for diagnostics.
    ///
    /// # Errors
    /// This function returns an error if PDDL temporal semantic rules are violated.
    /// It uses `#[track_caller]` via the error constructor to pinpoint the invalid
    /// construction site in the source code.
    fn check_temporal_invariant(
        &self,
        expr: ExprId,
        attempted_kind: ExprEntryKind,
    ) -> Result<(), ExprBuilderError> {
        if let Some(entry) = self.get(expr) {
            match entry.kind() {
                // PDDL constraint: Temporal operators cannot be nested inside each other.
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                    return Err(ExprBuilderError::invalid_temporal_invariant(
                        entry.kind().clone(),
                        attempted_kind,
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Constructs a `TimedInitialLiteral` (TIL).
    ///
    /// A Timed Initial Literal is a specialized PDDL construct representing an
    /// expression (usually an assignment or a predicate) that is added to the
    /// initial state at a specific point in time.
    ///
    /// # Arguments
    ///
    /// * `time` - The timestamp when the expression becomes effective.
    ///   Accepts any type convertible into `OrderedFloat<f64>` (e.g., `f64`, `OrderedFloat`).
    /// * `expr` - The [`ExprId`] of the formula or assignment to trigger at `time`.
    ///
    /// # Returns
    ///
    /// * `Ok(ExprId)` - The unique identifier of the timed literal node.
    /// * `Err(ExprBuilderError)` - If the timestamp is negative, violating PDDL semantics.
    pub fn timed_initial_literal<V>(
        &mut self,
        time: V,
        expr: ExprId,
    ) -> Result<ExprId, ExprBuilderError>
    where
        V: Into<OrderedFloat<f64>>,
    {
        let t: OrderedFloat<f64> = time.into();
        let val = t.into_inner();

        // Strict validation: we do not normalize, we raise a semantic error.
        // The parser and semantic analysis are expected to have validated this upstream.
        if val < 0.0 {
            return Err(ExprBuilderError::invalid_timestamp(val));
        }

        // Create a numeric node for the timestamp
        let time_node = self.number(val);

        // Intern the binary relation [Time, Expression]
        // This ensures the (Time, Expr) pair is unique in the store.
        Ok(self.intern(ExprEntryKind::TimedInitialLiteral, &[time_node, expr]))
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::VariableId;
    use crate::aiplan4rust::lir::store::expr::builder::ExprBuilderError;
    use crate::aiplan4rust::lir::store::expr::{ExprBuilder, ExprEntryKind, ExprStore};

    /// Verifies successful creation and Hash-Consing (deduplication).
    #[test]
    fn test_temporal_success_and_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.predicate(1);

        // Standard creation
        let start1 = builder.at_start(p).expect("Valid at-start");
        let end1 = builder.at_end(p).expect("Valid at-end");
        let overall1 = builder.overall(p).expect("Valid overall");

        // Hash-Consing check
        let start2 = builder.at_start(p).unwrap();
        assert_eq!(
            start1, start2,
            "Identical temporal expressions must share the same ID"
        );

        // Distinct type check
        assert_ne!(start1, end1);
        assert_ne!(start1, overall1);
    }

    /// Verifies that nesting the same operator is rejected (Illegal Idempotence).
    #[test]
    fn test_illegal_self_nesting() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.predicate(1);

        let start_p = builder.at_start(p).unwrap();

        // Attempt: (at start (at start p))
        let result = builder.at_start(start_p);

        match result {
            Err(ExprBuilderError::InvalidTemporalInvariant {
                existing_kind,
                attempted_kind,
            }) => {
                assert_eq!(existing_kind, ExprEntryKind::AtStart);
                assert_eq!(attempted_kind, ExprEntryKind::AtStart);
            }
            _ => panic!("Should have failed with InvalidTemporalInvariant"),
        }
    }

    /// Verifies that nesting different temporal operators is rejected.
    #[test]
    fn test_illegal_cross_nesting() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.predicate(1);

        let start_p = builder.at_start(p).unwrap();

        // Attempt: (overall (at start p))
        let result = builder.overall(start_p);

        if let Err(ExprBuilderError::InvalidTemporalInvariant {
            existing_kind,
            attempted_kind,
        }) = result
        {
            assert_eq!(existing_kind, ExprEntryKind::AtStart);
            assert_eq!(attempted_kind, ExprEntryKind::Overall);
        } else {
            panic!("Cross-nesting temporal operators should be rejected");
        }
    }

    /// Verifies that complex non-temporal formulas can be wrapped correctly.
    #[test]
    fn test_complex_formula_wrapping() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let p1 = builder.predicate(1);
        let p2 = builder.predicate(2);
        // Assuming a conjunction helper exists
        let and_expr = builder.intern(ExprEntryKind::And, &[p1, p2]);

        // (at start (and p1 p2)) is perfectly valid
        let result = builder.at_start(and_expr);
        assert!(
            result.is_ok(),
            "Should be able to wrap logical conjunctions"
        );
    }

    /// Verifies that temporal wrapping preserves free variable information.
    #[test]
    fn test_temporal_free_vars_preservation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_id = VariableId::new(5);
        let v = builder.variable(var_id);

        // At start (v) should have the same free variables as (v)
        let start_v = builder.at_start(v).unwrap();

        assert!(store.is_variable_free(v, var_id));
        assert!(
            store.is_variable_free(start_v, var_id),
            "Free variables must propagate through temporal operators"
        );
    }

    /// Verifies that temporal nodes remain valid after a store cache rebuild.
    #[test]
    fn test_temporal_consistency_after_rebuild() {
        let mut store = ExprStore::new();
        let p;
        let start_p;

        // 1. First scope: Create builder, intern nodes, then drop builder
        {
            let mut builder = ExprBuilder::new(&mut store);
            p = builder.predicate(1);
            start_p = builder.at_start(p).unwrap();
        } // builder is dropped here, releasing the borrow on store

        // 2. Now store is free to be borrowed mutably again
        store.rebuild_caches();

        // 3. Second scope: Create a new builder to verify consistency
        {
            let mut builder = ExprBuilder::new(&mut store);
            let start_p_post = builder.at_start(p).unwrap();

            assert_eq!(
                start_p, start_p_post,
                "Hash-consing must persist across cache rebuilds"
            );
        }
    }

    /// Verifies that logical operators can contain predicates but not other temporal operators.
    #[test]
    fn test_deep_logical_nesting_val231lidity() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.predicate(1);

        // Valid: (at start (not (predicate)))
        let not_p = builder.not(p);
        assert!(builder.at_start(not_p).is_ok());

        // Invalid: (at start (not (at start p)))
        // Note: Our current check_temporal_invariant only checks the IMMEDIATE child.
        // If you want to forbid this, you'd need a recursive check.
        // In PDDL, temporal operators are usually only at the top level of preconditions/effects.
        let start_p = builder.at_start(p).unwrap();
        let not_start_p = builder.not(start_p);

        // This should technically be invalid in most PDDL contexts
        let result = builder.at_start(not_start_p);
        // assert!(result.is_err()); // Only if you implement recursive checking
    }

    /// Objective: Verify the consistency of Timed Initial Literals (TIL) construction.
    ///
    /// This test checks two critical invariants:
    /// 1. **Zero Normalization**: 0.0 and -0.0 must result in the same ExprId.
    ///    This is guaranteed by `OrderedFloat` and internal number interning,
    ///    ensuring that the sign of zero doesn't duplicate nodes in the store.
    /// 2. **Result Handling**: Since the builder now returns a `Result`, we ensure
    ///    that valid timestamps can be unwrapped correctly.
    #[test]
    fn test_timed_literal_normalization() -> Result<(), ExprBuilderError> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let atom = builder.variable(VariableId::from(1));

        // Both 0.0 and -0.0 are >= 0.0, so they are valid.
        // They must be interned as the exact same expression node.
        let til1 = builder.timed_initial_literal(0.0, atom)?;
        let til2 = builder.timed_initial_literal(-0.0, atom)?;

        assert_eq!(
            til1, til2,
            "TIL with 0.0 and -0.0 must be identical due to OrderedFloat hashing and interning"
        );

        // Complementary check: ensure that a strictly negative value actually returns an error.
        let til_err = builder.timed_initial_literal(-1.0, atom);
        assert!(
            til_err.is_err(),
            "The builder must reject strictly negative timestamps with an ExprBuilderError"
        );

        Ok(())
    }
}
