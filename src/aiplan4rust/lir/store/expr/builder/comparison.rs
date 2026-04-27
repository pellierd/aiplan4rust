//! Comparison Expression Builder
//!
//! This module provides the implementation for constructing logical comparison
//! expressions (e.g., `=`, `<`, `<=`) within the [`ExprBuilder`].
//!
//! ## Design Principles
//!
//! The comparison logic is built upon three core pillars:
//!
//! 1. **Structural Canonicalization**: Reducing the variety of syntactic forms
//!    into a minimal set of canonical representations (e.g., always using `Less`
//!    instead of `Greater`) to facilitate efficient reasoning.
//!
//! 2. **Aggressive Deduplication (Hash-Consing)**: Ensuring that every
//!    unique comparison is stored exactly once. This transforms deep tree
//!    comparisons into simple pointer (ID) equalities.
//!
//! 3. **Static Evaluation (Folding)**: Resolving truth values at construction
//!    time whenever operands are constant literals, preventing the creation
//!    of redundant nodes in the [`ExprStore`].
//!
//! ## Transformation Pipeline
//!
//! Every comparison passes through a multi-stage pipeline:
//! **Canonicalize** → **Identity Check** → **Constant Fold** → **Intern**.

use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Constructs a functional comparison expression: `(op left right)`.
    ///
    /// This method orchestrates a highly optimized four-phase pipeline to ensure that the
    /// resulting [`ExprId`] points to a canonical, simplified, and unique representation
    /// of the comparison.
    ///
    /// By utilizing **Hash-Consing**, semantically equivalent comparisons (e.g., `x > 10`
    /// and `10 < x`) will always share the exact same [`ExprId`], enabling O(1)
    /// structural equality checks throughout the solver.
    ///
    /// # The Comparison Pipeline
    ///
    /// | Phase | Name | Responsibility |
    /// | :--- | :--- | :--- |
    /// | **1** | **Canonicalization** | Normalizes operators (e.g., `>` to `<`) and sorts operands for equality. |
    /// | **2** | **Identity Check** | Immediately resolves reflexive cases like `x == x` or `x < x`. |
    /// | **3** | **Constant Folding** | Statically evaluates comparisons between two numeric literals. |
    /// | **4** | **Interning** | Final deduplication in the [`ExprStore`] via the internal lookup table. |
    ///
    /// # Parameters
    ///
    /// * `op` - The [`CompareOp`] to apply (Equal, Less, Greater, etc.).
    /// * `left` - [`ExprId`] representing the left-hand side term.
    /// * `right` - [`ExprId`] representing the right-hand side term.
    ///
    /// # Returns
    ///
    /// * `ExprId` - A unique identifier for the normalized expression. This may point to
    ///   an existing node, a logical constant (`True`/`False`), or a newly created entry.
    ///
    /// # Design Note
    ///
    /// The operator is stored as part of the [`ExprEntryKind`] rather than a child node.
    /// This allows for rapid structural identification during pattern matching and
    /// reduces memory overhead by minimizing the number of edges in the expression graph.
    pub fn comparison(&mut self, op: CompareOp, left: ExprId, right: ExprId) -> ExprId {
        // Phase 1: Structural Normalization
        // Ensures that the logic engine only has to deal with a minimal set of operators.
        let (final_op, left, right) = self.canonicalize_comparison(op, left, right);

        // Phase 2: Trivial Identity Resolution (x op x)
        // Catch tautologies and contradictions early using pointer equality.
        if let Some(result) = self.simplify_identity(final_op, left, right) {
            return result;
        }

        // Phase 3: Numerical Constant Folding
        // Resolves literal comparisons (e.g., 5 < 10) to True/False.
        if let Some(result) = self.fold_comparison(final_op, left, right) {
            return result;
        }

        // Phase 4: Final Interning
        // Perform the final Hash-Consing lookup to guarantee uniqueness.
        self.intern(ExprEntryKind::Comparison(final_op), &[left, right])
    }

    /// Normalizes comparison operators and operands into a canonical form.
    ///
    /// This method implements **Phase 1 (Canonicalization)** of the comparison pipeline.
    /// Its primary goal is to ensure that semantically identical comparisons are
    /// represented by a single, unique structure. This is a prerequisite for effective
    /// **Hash-Consing**, as it allows different syntactic forms (e.g., `a > b` and `b < a`)
    /// to resolve to the same [`ExprId`].
    ///
    /// # Parameters
    ///
    /// * `op` - The original [`CompareOp`] provided by the user.
    /// * `l` - The [`ExprId`] of the left-hand operand.
    /// * `r` - The [`ExprId`] of the right-hand operand.
    ///
    /// # Return Value
    ///
    /// Returns a tuple `(CompareOp, ExprId, ExprId)` representing the normalized form:
    /// 1. **Operator Reduction**: `Greater` and `GreaterEq` are transformed into
    ///    `Less` and `LessEq` respectively by swapping the operands.
    /// 2. **Operand Ordering**: For the `Equal` operator, operands are sorted in
    ///    ascending order based on their [`ExprId`].
    ///
    /// # Transformation Logic
    ///
    /// | Input Operation | Normalized Output | Reasoning |
    /// | :--- | :--- | :--- |
    /// | `a > b` | `b < a` | Redundant operators reduction. |
    /// | `a >= b` | `b <= a` | Redundant operators reduction. |
    /// | `a == b` (where a > b) | `b == a` | Commutative sorting for uniqueness. |
    /// | `a < b` | `a < b` | Already in canonical form. |
    ///
    /// # Performance
    ///
    /// This function is marked `#[inline]` as it is a pure, branch-heavy function
    /// called at the entry point of every comparison. By reducing the operator
    /// space from five possible states to three (`Less`, `LessEq`, `Equal`),
    /// it significantly simplifies the logic for subsequent folding and interning phases.
    #[inline]
    fn canonicalize_comparison(
        &self,
        op: CompareOp,
        l: ExprId,
        r: ExprId,
    ) -> (CompareOp, ExprId, ExprId) {
        match op {
            // Normalize Greater-than relations by flipping them to Less-than.
            CompareOp::Greater => (CompareOp::Less, r, l),
            CompareOp::GreaterEq => (CompareOp::LessEq, r, l),

            // For Equality, apply commutative sorting based on ExprId.
            // This ensures (= A B) and (= B A) hash to the same value.
            CompareOp::Equal if r < l => (CompareOp::Equal, r, l),

            // Already canonical or no normalization applicable.
            _ => (op, l, r),
        }
    }

    /// Simplifies comparisons where both operands are identical.
    ///
    /// This method implements **Phase 2 (Identity Simplification)** of the comparison pipeline.
    /// It leverages the **Hash-Consing** invariant of the [`ExprStore`]: if two [`ExprId`]s
    /// are equal, the underlying expressions are guaranteed to be structurally identical.
    ///
    /// This allows for O(1) simplification of reflexive relations without inspecting
    /// the actual node content or performing numerical folding.
    ///
    /// # Parameters
    ///
    /// * `op` - The [`CompareOp`] to evaluate (e.g., Equal, Less, LessEq).
    /// * `l` - [`ExprId`] of the left-hand side.
    /// * `r` - [`ExprId`] of the right-hand side.
    ///
    /// # Return Value
    ///
    /// * `Some(ExprId)` - A logical constant if an identity was resolved:
    ///     - Returns [`self.empty_and()`] (True) for reflexive operators (`==`, `<=`).
    ///     - Returns [`self.empty_or()`] (False) for strict inequalities (`<`).
    /// * `None` - If operands are distinct (`l != r`), allowing the pipeline to proceed to
    ///   constant folding or interning.
    ///
    /// # Logical Logic
    ///
    /// The simplification follows standard mathematical properties:
    /// - `x == x` is always **True**.
    /// - `x <= x` is always **True**.
    /// - `x < x` is always **False**.
    ///
    /// # Performance
    ///
    /// This check is extremely cheap as it only involves an integer comparison of IDs.
    /// Executing this before the folding phase prevents unnecessary lookups in the
    /// entry store for identical variables or complex sub-expressions.
    #[inline]
    fn simplify_identity(&mut self, op: CompareOp, l: ExprId, r: ExprId) -> Option<ExprId> {
        if l == r {
            match op {
                // Reflexive cases: x = x and x <= x are tautologies.
                CompareOp::Equal | CompareOp::LessEq => Some(self.empty_and()),
                // Strict inequality: x < x is a contradiction.
                _ => Some(self.empty_or()),
            }
        } else {
            None
        }
    }

    /// Attempt to resolve the comparison at construction time if both operands are numeric constants.
    ///
    /// This method implements **Phase 3 (Constant Folding)** of the comparison pipeline.
    /// It utilizes epsilon-aware floating-point helpers to ensure that micro-imprecisions
    /// do not prevent logical simplifications.
    ///
    /// # Parameters
    ///
    /// * `op` - The [`CompareOp`] to evaluate (canonicalized).
    /// * `l` - [`ExprId`] of the left-hand side operand.
    /// * `r` - [`ExprId`] of the right-hand side operand.
    ///
    /// # Return Value
    ///
    /// * `Some(ExprId)` - Returns [`self.empty_and()`] (True) or [`self.empty_or()`] (False).
    /// * `None` - If operands are not numeric literals or involve **NaN**.
    ///
    /// # Floating-Point Robustness
    ///
    /// Unlike standard Rust comparisons, this method uses `is_eq`, `is_lt`, and `is_le`.
    /// This ensures that `x == y` resolves to `True` if the values differ by less than
    /// [`f64::EPSILON`], maintaining consistency with temporal constraints (e.g., `hold_during`).
    #[inline]
    fn fold_comparison(&mut self, op: CompareOp, l: ExprId, r: ExprId) -> Option<ExprId> {
        let l_node = self.get(l)?;
        let r_node = self.get(r)?;

        if let (ExprEntryKind::Number(lv), ExprEntryKind::Number(rv)) =
            (l_node.kind(), r_node.kind())
        {
            let (l_val, r_val) = (lv.into_inner(), rv.into_inner());

            // Skip folding for NaN values to preserve semantic error propagation.
            if l_val.is_nan() || r_val.is_nan() {
                return None;
            }

            // Phase 3 Evaluation: Using robust epsilon-aware helpers.
            let truth = match op {
                CompareOp::Equal => self.is_eq(l_val, r_val),
                CompareOp::Less => self.is_lt(l_val, r_val),
                CompareOp::LessEq => self.is_le(l_val, r_val),
                // Safety: Greater and GreaterEq are handled by canonicalization phase.
                _ => unsafe { std::hint::unreachable_unchecked() },
            };

            return Some(if truth {
                self.empty_and()
            } else {
                self.empty_or()
            });
        }
        None
    }

    /// Constructs an equality comparison: `(= left right)`.
    ///
    /// This is a commutative operation. The underlying pipeline automatically
    /// sorts the operands by their [`ExprId`] to ensure that `(= a b)` and `(= b a)`
    /// result in the same canonical expression.
    ///
    /// # Parameters
    ///
    /// * `left` - The [`ExprId`] of the first term to compare.
    /// * `right` - The [`ExprId`] of the second term to compare.
    ///
    /// # Returns
    ///
    /// * [`ExprId`] - The unique identifier for the resulting equality expression.
    ///   May return a logical constant if both sides are identical or numeric literals.
    pub fn equal(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Equal, left, right)
    }

    /// Constructs a strict "less-than" comparison: `(< left right)`.
    ///
    /// # Parameters
    ///
    /// * `left` - The [`ExprId`] of the value expected to be smaller.
    /// * `right` - The [`ExprId`] of the value expected to be larger.
    ///
    /// # Returns
    ///
    /// * [`ExprId`] - The unique identifier for the normalized `Less` expression.
    pub fn less(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Less, left, right)
    }

    /// Constructs a "less-than-or-equal" comparison: `(<= left right)`.
    ///
    /// # Parameters
    ///
    /// * `left` - The [`ExprId`] of the value expected to be smaller or equal.
    /// * `right` - The [`ExprId`] of the value expected to be larger or equal.
    ///
    /// # Returns
    ///
    /// * [`ExprId`] - The unique identifier for the normalized `LessEq` expression.
    pub fn less_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::LessEq, left, right)
    }

    /// Constructs a strict "greater-than" comparison: `(> left right)`.
    ///
    /// # Canonicalization
    ///
    /// Note: This is automatically normalized to a `Less` comparison by swapping
    /// the operands: `(> a b)` results in an [`ExprId`] pointing to `(< b a)`.
    ///
    /// # Parameters
    ///
    /// * `left` - The [`ExprId`] of the value expected to be larger.
    /// * `right` - The [`ExprId`] of the value expected to be smaller.
    ///
    /// # Returns
    ///
    /// * [`ExprId`] - The unique identifier for the normalized `Less` expression.
    pub fn greater(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Greater, left, right)
    }

    /// Constructs a "greater-than-or-equal" comparison: `(>= left right)`.
    ///
    /// # Canonicalization
    ///
    /// Note: This is automatically normalized to a `LessEq` comparison by swapping
    /// the operands: `(>= a b)` results in an [`ExprId`] pointing to `(<= b a)`.
    ///
    /// # Parameters
    ///
    /// * `left` - The [`ExprId`] of the value expected to be larger or equal.
    /// * `right` - The [`ExprId`] of the value expected to be smaller or equal.
    ///
    /// # Returns
    ///
    /// * [`ExprId`] - The unique identifier for the normalized `LessEq` expression.
    pub fn greater_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::GreaterEq, left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{CompareOp, FunctionSkeletonId, FunctionSymbolId, VariableId};
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Objective: Ensure 'Greater' is normalized to 'Less' and operands are swapped.
    /// Input: Calling builder.greater(a, b).
    /// Output: An ExprId pointing to a Comparison(Less) with children [b, a].
    #[test]
    fn test_comparison_canonicalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));

        let id = builder.greater(a, b);

        let entry = store.get(id).expect("Entry must exist");
        assert_eq!(entry.kind(), &ExprEntryKind::Comparison(CompareOp::Less));
        assert_eq!(
            entry.children(),
            &[b, a],
            "Operands must be swapped for canonical Less form"
        );
    }

    /// Objective: Ensure equality operands are sorted by ExprId to guarantee a single canonical form.
    /// Input: Calling builder.equal(max_id, min_id).
    /// Output: Comparison(Equal) with children sorted as [min_id, max_id].
    #[test]
    fn test_comparison_equality_sorting() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let (min_id, max_id) = if v1 < v2 { (v1, v2) } else { (v2, v1) };

        let id = builder.equal(max_id, min_id);

        let entry = store.get(id).expect("Entry must exist");
        assert_eq!(
            entry.children(),
            &[min_id, max_id],
            "Equality operands must be sorted by ID"
        );
    }

    /// Objective: Verify that self-comparisons bypass entry creation and return logical constants.
    /// Input: less_eq(a, a) and less(a, a).
    /// Output: Exact matches with empty_and (True) and empty_or (False) IDs.
    #[test]
    fn test_comparison_identity_simplification() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));

        let f1 = builder.less_eq(a, a);
        let f2 = builder.less(a, a);

        assert_eq!(
            f1,
            builder.empty_and(),
            "x <= x must be the 'True' constant"
        );
        assert_eq!(f2, builder.empty_or(), "x < x must be the 'False' constant");
    }

    /// Objective: Evaluate numerical comparisons at construction time to avoid storing redundant nodes.
    /// Input: Calling builder.less(10.0, 20.0).
    /// Output: The 'True' constant ID (empty_and) without adding a Comparison node to the store.
    #[test]
    fn test_comparison_constant_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create two constant numbers
        let n10 = builder.number(10.0);
        let n20 = builder.number(20.0);

        // Perform the comparison (10 < 20)
        let id = builder.less(n10, n20);

        // 1. Check the logical result (must be True/empty_and)
        assert_eq!(id, builder.empty_and(), "10 < 20 must fold to True");

        // 2. Check that no Comparison entry was actually created in the entries vector
        // We use a range and `store.get()` since we have that public API
        let mut has_comparison = false;
        for i in 0..store.len() {
            // ExprId::new(i) matches your internal indexing
            if let Some(node) = store.get(ExprId::new(i)) {
                if matches!(node.kind(), ExprEntryKind::Comparison(_)) {
                    has_comparison = true;
                    break;
                }
            }
        }

        assert!(
            !has_comparison,
            "Store should not contain a Comparison node for folded constants"
        );
    }

    /// Objective: Verify that identical comparison logic shares the exact same ExprId and memory slot.
    /// Input: Two separate calls to builder.less(a, 5.0).
    /// Output: Identical ExprId and the store size increases only by one.
    #[test]
    fn test_comparison_structural_deduplication() {
        let mut store = ExprStore::new();

        // 1. We take the count BEFORE creating the builder
        let count_before = store.len();

        {
            let mut builder = ExprBuilder::new(&mut store);
            let a = builder.variable(VariableId::from(1));
            let b = builder.number(5.0);

            // Here we use the builder
            let f1 = builder.less(a, b);
            let f2 = builder.less(a, b);

            assert_eq!(f1, f2, "Redundant calls must return the same ExprId");

            // Validation of content inside the builder scope
            let entry = builder.get(f1).unwrap();
            assert_eq!(entry.kind(), &ExprEntryKind::Comparison(CompareOp::Less));
            assert_eq!(entry.children(), &[a, b]);
        }
        // builder is dropped here, &mut store is released

        // 2. We can now borrow store again to check the final count
        let count_after = store.len();
        assert_eq!(
            count_after,
            count_before + 3,
            "Should be: 1 var + 1 num + 1 comparison"
        );
    }

    /// Objective: Verify that NaN values do not cause crashes and are handled safely without incorrect folding.
    /// Input: Comparison involving f64::NAN via builder.less(nan, 10.0).
    /// Output: A stored Comparison node (folding is skipped to avoid IEEE-754 NaN comparison traps).
    #[test]
    fn test_comparison_nan_handling() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let n_nan = builder.number(f64::NAN);
        let n_10 = builder.number(10.0);

        let id = builder.less(n_nan, n_10);

        let entry = store.get(id).expect("Entry must exist");
        assert!(matches!(entry.kind(), ExprEntryKind::Comparison(CompareOp::Less)),
                    "NaN comparisons should result in a stored node rather than folding to True/False constants");
    }

    /// Objective: Ensure that different operators with the same operands result in distinct ExprIds.
    /// Input: Calling builder.less(a, b) and builder.less_eq(a, b).
    /// Output: Two different ExprIds (f1 != f2).
    #[test]
    fn test_operator_distinction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));

        let f1 = builder.less(a, b);
        let f2 = builder.less_eq(a, b);

        assert_ne!(
            f1, f2,
            "Distinct operators (< vs <=) must have distinct ExprIds"
        );
    }

    /// Objective: Verify that commutativity and normalization work for complex nested terms like atomic formulas.
    /// Input: (atom1 = atom2) and (atom2 = atom1).
    /// Output: Identical ExprId due to operand sorting during normalization.
    #[test]
    fn test_complex_structural_equality() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Setup complex atoms
        let sym = FunctionSymbolId::from(1);
        let skel = FunctionSkeletonId::from(10);
        let arg = builder.variable(VariableId::from(1));

        let atom1 = builder.function_term(sym, &[arg], skel);
        let atom2 = builder.function_term(sym, &[arg], skel); // Should be deduped already

        let id1 = builder.equal(atom1, atom2);
        let id2 = builder.equal(atom2, atom1);

        assert_eq!(
            id1, id2,
            "Equality of complex terms must be canonicalized regardless of operand order"
        );
    }

    /// Objective: Check if the store correctly rebuilds its lookup table and maintains deduplication after clear().
    /// Input: Interning identical comparisons before and after a store.clear() call.
    /// Output: New entries are correctly deduped in the fresh store state.
    #[test]
    fn test_store_clear_integrity() {
        let mut store = ExprStore::new();
        {
            let mut builder = ExprBuilder::new(&mut store);
            let n1 = builder.number(1.0);
            let n2 = builder.number(2.0);
            builder.less(n1, n2);
        }

        store.clear();

        // CHANGEMENT ICI : Le store contient TRUE et FALSE par défaut
        assert_eq!(
            store.len(),
            2,
            "Store must contain the 2 default constants (TRUE/FALSE) after clear"
        );

        let mut builder = ExprBuilder::new(&mut store);

        // Ces IDs seront probablement 2 et 3
        let n1 = builder.number(1.0);
        let n2 = builder.number(2.0);

        let f1 = builder.less(n1, n2);
        let f2 = builder.less(n1, n2);

        assert_eq!(
            f1, f2,
            "Deduplication must function correctly in a cleared and reused store"
        );
    }

    /// Objective: Ensure GreaterEqual is normalized to LessEqual with swapped operands.
    /// Input: builder.greater_eq(a, b).
    /// Output: Comparison(LessEqual) with children [b, a].
    #[test]
    fn test_greater_equal_normalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));

        let id = builder.greater_eq(a, b);

        let entry = store.get(id).expect("Entry must exist");
        assert_eq!(entry.kind(), &ExprEntryKind::Comparison(CompareOp::LessEq));
        assert_eq!(
            entry.children(),
            &[b, a],
            "GreaterEqual must flip to LessEqual"
        );
    }

    /// Objective: Verify that equality between different node types (Variable and Number) is canonicalized.
    /// Input: (= var 10.0) and (= 10.0 var).
    /// Output: Identical ExprId.
    #[test]
    fn test_mixed_type_equality_canonicalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let num = builder.number(10.0);

        let id1 = builder.equal(var, num);
        let id2 = builder.equal(num, var);

        assert_eq!(
            id1, id2,
            "Equality must be commutative regardless of node types"
        );
    }

    /// Objective: Verify that Hash-Consing still works after a store has been cleared and refilled,
    /// specifically checking that ID generation doesn't collide.
    #[test]
    fn test_id_collision_after_clear() {
        let mut store = ExprStore::new();
        let id_first_run;
        {
            let mut builder = ExprBuilder::new(&mut store);
            id_first_run = builder.number(42.0);
        }

        store.clear();

        let mut builder = ExprBuilder::new(&mut store);
        let id_second_run = builder.number(42.0);

        // Logical check: After clear, the first inserted element should likely get the same ID
        // as the first element of the previous run (usually index 0).
        assert_eq!(
            id_first_run, id_second_run,
            "IDs should be reused/consistent after clear"
        );
    }

    /// Objective: Ensure that constant folding handles -0.0 and 0.0 as the same value.
    /// Input: less(0.0, -0.0).
    /// Output: False (empty_or) because 0.0 is not less than -0.0.
    #[test]
    fn test_zero_sign_normalization_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p_zero = builder.number(0.0);
        let n_zero = builder.number(-0.0);

        // If normalized, this is 0.0 < 0.0 => False
        let id = builder.less(p_zero, n_zero);

        assert_eq!(
            id,
            builder.empty_or(),
            "Comparison should treat 0.0 and -0.0 as identical"
        );
    }

    /// Objective: Ensure that the interning logic is perfectly deterministic and
    /// independent of the store's previous history (as long as the result is the same).
    #[test]
    fn test_deep_canonical_determinism() {
        let mut store1 = ExprStore::new();
        let mut store2 = ExprStore::new();

        // Scenario 1: Direct creation
        let id1 = {
            let mut b = ExprBuilder::new(&mut store1);
            let v = b.variable(VariableId::from(1));
            let n = b.number(10.0);
            b.equal(v, n)
        };

        // Scenario 2: Noise, Clear, then creation
        {
            let mut b = ExprBuilder::new(&mut store2);
            b.number(99.0);
            b.variable(VariableId::from(99));
        }
        store2.clear(); // On repart à "zéro"

        let id2 = {
            let mut b = ExprBuilder::new(&mut store2);
            let v = b.variable(VariableId::from(1));
            let n = b.number(10.0);
            b.equal(v, n)
        };

        assert_eq!(
            id1, id2,
            "The same expression must always result in the same ExprId (index 2)"
        );

        // Bonus: Verify that content is equal bit by bit
        let e1 = store1.get(id1).unwrap();
        let e2 = store2.get(id2).unwrap();
        assert_eq!(format!("{:?}", e1.kind()), format!("{:?}", e2.kind()));
    }
}
