//! # Assignment Module: State Transition & Effect Construction
//!
//! This module provides an API for constructing functional assignment expressions,
//! also known as "effects" in the context of AI planning. It manages how fluents
//! (state variables) are modified during state transitions.
//!
//! ## Overview
//!
//! The assignment builder ensures that all state-changing operations are interned
//! in a canonical and optimized form. By intercepting "No-Op" operations early,
//! it prevents the pollution of the LIR store with redundant nodes, simplifying
//! the task for subsequent grounding and solving phases.
//!
//! ## Optimization Strategies
//!
//! ### 1. Identity Elision (No-Op Removal)
//! The builder identifies operations that result in a mathematical identity and
//! collapses them into a neutral `True` node (empty conjunction):
//! - **Additive Identity**: `increase` or `decrease` by `0.0`.
//! - **Multiplicative Identity**: `scale-up` or `scale-down` by `1.0`.
//!
//! ### 2. Algebraic Reduction
//! To maintain a lean representation, certain operations are re-encoded into
//! simpler forms:
//! - **Zero Scaling**: Scaling a fluent by `0.0` is automatically converted into
//!   a direct `assign` of `0.0`.
//!
//! ### 3. Efficient Lookups
//! Using the internal `pub(crate) get_number` helper, the builder performs
//! zero-allocation checks on numeric literals. This ensures that constants are
//! folded or elided before any hashing or interning occurs.
//!
//! ## Example
//!
//! ```ignore
//! // PDDL: (increase (battery-level) 0)
//! // Result: A neutral node (True), as adding 0 has no effect.
//! let effect = builder.increase(battery_id, zero_id);
//! ```

use crate::aiplan4rust::lang::AssignOp;
use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Creates a functional assignment node: `(op target value)`.
    ///
    /// This method handles various assignment types (e.g., `assign`, `increase`, `scale-up`)
    /// and applies preemptive optimizations to elide redundant operations and simplify
    /// algebraic edge cases using epsilon-aware floating-point logic.
    ///
    /// # Optimizations
    ///
    /// 1. **Identity Elision (No-Op)**:
    ///    Detects operations that have no significant effect on the state (within epsilon)
    ///    and replaces them with a neutral `True` node (empty conjunction).
    ///    - `increase` or `decrease` by a value that `is_zero`.
    ///    - `scale-up` or `scale-down` by a value that `is_eq` to `1.0`.
    ///
    /// 2. **Algebraic Reduction & Totalization**:
    ///    - **Zero Scaling**: Scaling a fluent by ~`0.0` (`ScaleUp`) is re-encoded
    ///      as a direct `Assign` of `0.0`.
    ///    - **Safety (Zero Division)**: Dividing a fluent by ~`0.0` (`ScaleDown`) is
    ///      re-encoded as an `Assign` of `NaN`. This prevents numerical explosions
    ///      and ensures consistent error propagation.
    ///
    /// 3. **Numerical Robustness**:
    ///    By using `is_zero` and `is_eq`, the builder avoids creating thousands of
    ///    redundant nodes for micro-increments (e.g., `1e-18`) that exceed the
    ///    precision limits of the solver.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`AssignOp`] characterizing the effect (Assign, Increase, ScaleUp, etc.).
    /// * `target` - The [`ExprId`] of the fluid or variable being modified.
    /// * `value` - The [`ExprId`] of the expression being applied to the target.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the interned assignment, or a neutral node
    ///   if the operation was simplified away.
    pub fn assignment(&mut self, op: AssignOp, target: ExprId, value: ExprId) -> ExprId {
        // 1. Fast path for numeric literals
        if let Some(val) = self.get_number(value) {
            // 2. Identify No-Op operations using robust epsilon-aware helpers
            let is_no_op = match op {
                // Adding/Subtracting nearly 0.0 results in no change
                AssignOp::Increase | AssignOp::Decrease => self.is_zero(val),
                // Multiplying/Dividing by nearly 1.0 results in no change
                AssignOp::ScaleUp | AssignOp::ScaleDown => self.is_eq(val, 1.0),
                AssignOp::Assign => false,
            };

            if is_no_op {
                // Return an empty conjunction (True) to signify no effect.
                return self.empty_and();
            }

            // 3. Algebraic optimization & Safety (Handling near-zero values)
            if self.is_zero(val) {
                match op {
                    // Multiplier is nearly 0.0: f = f * 0  => f = 0
                    AssignOp::ScaleUp => {
                        let zero = self.number(0.0);
                        return self
                            .intern(ExprEntryKind::Assignment(AssignOp::Assign), &[target, zero]);
                    }
                    // Divisor is nearly 0.0: f = f / 0  => f = NaN (Safe Totalization)
                    AssignOp::ScaleDown => {
                        let nan = self.number(f64::NAN);
                        return self
                            .intern(ExprEntryKind::Assignment(AssignOp::Assign), &[target, nan]);
                    }
                    _ => {}
                }
            }
        }

        // 4. Standard Interning
        // If no optimizations apply, the assignment is interned into the store.
        self.intern(ExprEntryKind::Assignment(op), &[target, value])
    }

    /// Creates an assignment effect: `(assign target value)`.
    ///
    /// This represents a direct state change where the target fluent is set to the
    /// result of the provided value expression.
    ///
    /// # Arguments
    ///
    /// * `target` - An [`ExprId`] representing the fluent or variable to be modified.
    /// * `value` - An [`ExprId`] representing the expression whose result will be assigned.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the interned assignment expression.
    pub fn assign(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assignment(AssignOp::Assign, target, value)
    }

    /// Creates an incremental effect: `(increase target value)`.
    ///
    /// The target fluent's value will be increased by the result of the value expression.
    ///
    /// # Arguments
    ///
    /// * `target` - The [`ExprId`] of the fluent to increase.
    /// * `value` - The [`ExprId`] of the increment amount.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the assignment, or a neutral `True` node if `value` is `0.0`.
    pub fn increase(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assignment(AssignOp::Increase, target, value)
    }

    /// Creates a decremental effect: `(decrease target value)`.
    ///
    /// The target fluent's value will be decreased by the result of the value expression.
    ///
    /// # Arguments
    ///
    /// * `target` - The [`ExprId`] of the fluent to decrease.
    /// * `value` - The [`ExprId`] of the decrement amount.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the assignment, or a neutral `True` node if `value` is `0.0`.
    pub fn decrease(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assignment(AssignOp::Decrease, target, value)
    }

    /// Creates a scaling effect (multiplication): `(scale-up target value)`.
    ///
    /// Multiplies the target fluent by the result of the value expression.
    ///
    /// # Arguments
    ///
    /// * `target` - The [`ExprId`] of the fluent to scale.
    /// * `value` - The [`ExprId`] of the multiplier.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the scaling effect, a neutral node if `value` is `1.0`,
    ///   or a direct `Assign(0.0)` if `value` is `0.0`.
    pub fn scale_up(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assignment(AssignOp::ScaleUp, target, value)
    }

    /// Creates a scaling effect (division): `(scale-down target value)`.
    ///
    /// Divides the target fluent by the result of the value expression.
    ///
    /// # Arguments
    ///
    /// * `target` - The [`ExprId`] of the fluent to scale down.
    /// * `value` - The [`ExprId`] of the divisor.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the scaling effect, or a neutral node if `value` is `1.0`.
    pub fn scale_down(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assignment(AssignOp::ScaleDown, target, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, VariableId};
    use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Test: (increase f1 0.0) -> empty_and
    /// Description: Verifies that additive and multiplicative identity operations
    /// (No-Ops) are elided and replaced by a neutral 'True' node.
    #[test]
    fn test_assignment_no_op_elision() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));
        let true_node = builder.empty_and();

        // 1. Additive Identity (0.0)
        let zero = builder.number(0.0);
        assert_eq!(
            builder.increase(target, zero),
            true_node,
            "Increase by 0.0 must be elided to a neutral node."
        );
        assert_eq!(
            builder.decrease(target, zero),
            true_node,
            "Decrease by 0.0 must be elided to a neutral node."
        );

        // 2. Multiplicative Identity (1.0)
        let one = builder.number(1.0);
        assert_eq!(
            builder.scale_up(target, one),
            true_node,
            "Scale-up by 1.0 must be elided to a neutral node."
        );
        assert_eq!(
            builder.scale_down(target, one),
            true_node,
            "Scale-down by 1.0 must be elided to a neutral node."
        );
    }

    /// Test: (scale-up f1 0.0) -> (assign f1 0.0)
    /// Description: Verifies that scaling a fluent by zero is reduced to a
    /// direct assignment, simplifying the effect for the solver.
    #[test]
    fn test_assignment_algebraic_reduction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));
        let zero = builder.number(0.0);

        // Input: scale-up target 0.0
        let effect = builder.scale_up(target, zero);

        let node = builder.get(effect).expect("Effect node should exist");
        if let ExprEntryKind::Assignment(op) = node.kind() {
            assert_eq!(
                *op,
                AssignOp::Assign,
                "The 'scale-up by 0' operation should be reduced to a direct 'assign'."
            );
            assert_eq!(
                node.children()[1],
                zero,
                "The assigned value must be the numeric literal 0.0."
            );
        } else {
            panic!("Expected an Assignment node, but found a different ExprEntryKind.");
        }
    }

    /// Test: (assign f1 42.0) == (assign f1 42.0)
    /// Description: Validates that the store correctly deduplicates identical
    /// assignment operations using Hash-Consing.
    #[test]
    fn test_assignment_interning_and_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));
        let val = builder.number(42.0);

        // Standard interned assignments
        let a1 = builder.assign(target, val);
        let a2 = builder.assign(target, val);

        // Verification of Pointer Equality (Hash-Consing)
        assert_eq!(
            a1, a2,
            "Identical assignments must share the same ExprId to minimize memory footprint."
        );

        let node = builder.get(a1).unwrap();
        assert!(matches!(
            node.kind(),
            ExprEntryKind::Assignment(AssignOp::Assign)
        ));
        assert_eq!(node.children()[0], target);
        assert_eq!(node.children()[1], val);
    }

    /// Test: (assign f1 f1) -> Assignment(Assign, [f1, f1])
    /// Description: Verifies that self-assignment is correctly interned as a valid node.
    /// Note: This preserves the assignment in the LIR even if the value matches the target.
    #[test]
    fn test_self_assignment_interning() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));

        let effect = builder.assign(target, target);

        let node = builder.get(effect).unwrap();
        assert!(matches!(
            node.kind(),
            ExprEntryKind::Assignment(AssignOp::Assign)
        ));
        assert_eq!(node.children()[0], node.children()[1]);
    }

    /// Test: (increase target x) -> Assignment(Increase, [target, x])
    /// Description: Ensures that symbolic variables prevent elision.
    /// Since 'x' is unknown at construction time, the operation must be preserved.
    #[test]
    fn test_symbolic_no_elision() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));
        let x = builder.variable(VariableId::from(2));

        let effect = builder.increase(target, x);

        assert_ne!(
            effect,
            builder.empty_and(),
            "Symbolic increase must not be elided as the value is not statically zero."
        );
        let node = builder.get(effect).unwrap();
        assert!(matches!(
            node.kind(),
            ExprEntryKind::Assignment(AssignOp::Increase)
        ));
    }

    /// Test: (assign target (+ 1.0 1.0)) vs (assign target 2.0) -> Same ExprId
    /// Description: Verifies that Hash-Consing works across folded expressions.
    /// Both assignments should point to the same unique ID in the store.
    #[test]
    fn test_deep_assignment_deduplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));

        // Construction 1: 1.0 + 1.0 (Automatically folded to 2.0)
        let n1 = builder.number(1.0);
        let val1 = builder.arithmetic(ArithmeticOp::Add, &[n1, n1]);
        let assign1 = builder.assign(target, val1);

        // Construction 2: Literal 2.0
        let val2 = builder.number(2.0);
        let assign2 = builder.assign(target, val2);

        assert_eq!(
            assign1, assign2,
            "Assignments must be identical after the values are constant-folded."
        );
    }

    /// Test: (scale-down target 0.0) -> (assign target NaN)
    /// Description: Verifies "Safe Totalization". Dividing by zero is transformed
    /// into a direct assignment of NaN to catch errors early in the pipeline.
    #[test]
    fn test_scale_down_by_zero_to_assign_nan() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));
        let zero = builder.number(0.0);

        let effect = builder.scale_down(target, zero);
        let node = builder.get(effect).expect("Assignment node should exist");

        // Check transformation to Assign
        assert!(
            matches!(node.kind(), ExprEntryKind::Assignment(AssignOp::Assign)),
            "Scale-down by zero must be reduced to a direct assignment."
        );

        // Check value is NaN
        let val_id = node.children()[1];
        assert!(
            builder.get_number(val_id).unwrap().is_nan(),
            "The assigned value must be NaN to represent the division by zero error."
        );
    }

    /// Test: (increase target -0.0) -> empty_and
    /// Description: Ensures that negative zero is treated as a neutral element,
    /// preventing the creation of redundant nodes for signed zero values.
    #[test]
    fn test_negative_zero_elision() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));

        let neg_zero = builder.number(-0.0);
        // Should be elided just like 0.0
        assert_eq!(builder.increase(target, neg_zero), builder.empty_and());
    }

    /// Test: (increase target (+ 5.0 -5.0)) -> empty_and
    /// Description: Verifies that the builder captures neutral elements even when
    /// they result from a complex arithmetic folding operation.
    #[test]
    fn test_folded_neutral_elision() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));

        // (+ 5.0 -5.0) folds to 0.0
        let plus_5 = builder.number(5.0);
        let minus_5 = builder.number(-5.0);
        let val = builder.add(&[plus_5, minus_5]);

        // increase target 0.0 -> empty_and
        assert_eq!(builder.increase(target, val), builder.empty_and());
    }

    /// Test: (increase f1 f1) -> Assignment(Increase, [f1, f1])
    /// Description: Ensures that increasing a fluent by itself is preserved,
    /// as it is a valid state transformation (doubling the value).
    #[test]
    fn test_self_increment_preservation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let target = builder.variable(VariableId::from(1));

        let effect = builder.increase(target, target);
        assert_ne!(
            effect,
            builder.empty_and(),
            "Self-increment is not a No-Op."
        );
    }
}
