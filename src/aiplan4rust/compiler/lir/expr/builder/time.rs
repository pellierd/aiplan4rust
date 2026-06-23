//! # Temporal Operators Module
//!
//! Provides constructors for PDDL 2.1 temporal constraints: `at start`, `at end`,
//! and `overall`. These operators are essential for defining when conditions
//! must hold or when effects occur in temporal planning.
//!
//! This module implements structural optimizations to prevent nested temporal
//! redundancy and ensures high-performance interning through inlining.

use crate::aiplan4rust::compiler::lir::expr::builder::{ExprBuilder, ExprBuilderError};
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind};
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
        let kind = ExprKind::AtStart;
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
        let kind = ExprKind::AtEnd;
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
        let kind = ExprKind::Overall;
        self.check_temporal_invariant(expr, kind.clone())?;
        Ok(self.intern(kind, &[expr]))
    }

    /// Validates that the target expression is not an instance of any PDDL temporal
    /// operator (`AtStart`, `AtEnd`, `Overall`) to prevent illegal direct or cross-nesting.
    ///
    /// This check strictly targets the immediate child node to catch illegal temporal
    /// wrappers right at the boundary before interning.
    ///
    /// # Performance
    /// Wrapped completely inside a `cfg!(debug_assertions)` block. In release builds,
    /// this check is completely stripped out by the compiler, reducing the function
    /// call overhead to a zero-cost `Ok(())`.
    ///
    /// # Arguments
    /// * `expr` - The `ExprId` of the immediate expression to be checked.
    /// * `attempted_kind` - The `ExprKind` of the temporal operator being applied.
    ///
    /// # Returns
    /// * `Ok(())` - If the expression is not a temporal operator, or if running in release mode.
    /// * `Err(ExprBuilderError::InvalidTemporalInvariant)` - If any temporal operator is detected in debug mode.
    fn check_temporal_invariant(
        &self,
        expr: ExprId,
        attempted_kind: ExprKind,
    ) -> Result<(), ExprBuilderError> {
        if cfg!(debug_assertions) {
            if let Some(entry) = self.get(expr) {
                match entry.kind() {
                    // Interdit immédiatement n'importe quel autre opérateur temporel
                    ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        return Err(ExprBuilderError::invalid_temporal_invariant(
                            *entry.kind(),
                            attempted_kind,
                        ));
                    }
                    _ => {}
                }
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
        // This ensures the (Time, Expr) pair is unique in the old.
        Ok(self.intern(ExprKind::TimedInitialLiteral, &[time_node, expr]))
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilderError;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::VariableId;

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

        if cfg!(debug_assertions) {
            // En mode Debug, on DOIT rejeter l'imbrication
            match result {
                Err(ExprBuilderError::InvalidTemporalInvariant {
                    existing_kind,
                    attempted_kind,
                }) => {
                    assert_eq!(existing_kind, ExprKind::AtStart);
                    assert_eq!(attempted_kind, ExprKind::AtStart);
                }
                _ => panic!("Should have failed with InvalidTemporalInvariant in debug mode"),
            }
        } else {
            // En mode Release, la vérification est désactivée : l'expression doit réussir
            assert!(
                result.is_ok(),
                "Should succeed in release mode since invariants are skipped"
            );
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

        if cfg!(debug_assertions) {
            // En mode Debug, on DOIT lever une erreur
            if let Err(ExprBuilderError::InvalidTemporalInvariant {
                existing_kind,
                attempted_kind,
            }) = result
            {
                assert_eq!(existing_kind, ExprKind::AtStart);
                assert_eq!(attempted_kind, ExprKind::Overall);
            } else {
                panic!("Cross-nesting temporal operators should be rejected in debug mode");
            }
        } else {
            // En mode Release, l'expression passe sans encombre
            assert!(result.is_ok(), "Should succeed in release mode");
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
        let and_expr = builder.intern(ExprKind::And, &[p1, p2]);

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

    /// Verifies that temporal nodes remain valid after a old cache rebuild.
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
        } // builder is dropped here, releasing the borrow on old

        // 2. Now old is free to be borrowed mutably again
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

    /// Verifies that logical operators can contain predicates and that direct
    /// structural temporal nesting is blocked immediately at the first level.
    #[test]
    fn test_deep_logical_nesting_val231lidity() {
        let mut store = ExprStore::new();

        let (not_p, start_p) = {
            let mut builder = ExprBuilder::new(&mut store);
            let p = builder.predicate(1);

            let not_p = builder.not(p);
            let start_p = builder.at_start(p).unwrap();

            (not_p, start_p)
        };

        let mut builder = ExprBuilder::new(&mut store);

        // Valide dans tous les modes (pas d'imbrication directe sous le AtStart)
        assert!(builder.at_start(not_p).is_ok());

        // Invalide en Debug / Valide en Release
        let result = builder.overall(start_p);

        if cfg!(debug_assertions) {
            assert!(
                result.is_err(),
                "Temporal operators must not be nested directly under another temporal operator"
            );
        } else {
            assert!(result.is_ok(), "Invariants are ignored in release mode");
        }
    }

    /// Objective: Verify the consistency of Timed Initial Literals (TIL) construction.
    ///
    /// This test checks two critical invariants:
    /// 1. **Zero Normalization**: 0.0 and -0.0 must result in the same ExprId.
    ///    This is guaranteed by `OrderedFloat` and internal number interning,
    ///    ensuring that the sign of zero doesn't duplicate nodes in the old.
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
