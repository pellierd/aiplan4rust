//! # Trajectory Constraints (PDDL 3.0)
//!
//! This module provides the implementation for constructing temporal trajectory
//! constraints as defined in the PDDL 3.0 specification.
//!
//! ## Design Principles
//!
//! The construction of trajectory constraints follows three main optimization rules:
//!
//! 1. **Structural Integrity**: All constraints are interned in the [`ExprStore`] to
//!    ensure that identical constraints share the same [`ExprId`] (Hash-Consing).
//!
//! 2. **Early Folding (Tautology/Contradiction)**: Constraints that can be
//!    statically resolved at construction time (e.g., `always(True)`) are simplified
//!    immediately to reduce the expression graph's depth.
//!
//! 3. **Vacuous Truth Handling**: Based on the logic of temporal implications,
//!    constraints with an impossible trigger (e.g., `sometime-after(False, ...)`
//!    are resolved to `True` during construction.
//!
//! ## Transformation Pipeline
//!
//! Each method follows a strict validation-before-interning logic:
//! **Validation** (Is interval valid?) → **Folding** (Is it trivially True/False?) → **Interning**.

use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Constructs an `always` constraint: the condition must hold in every single state
    /// of the plan's trajectory.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Tautology Folding**: If `expr` is already logically `True`, the requirement
    ///    that it must "always" be true is redundant. Returns `True`.
    ///
    /// # Parameters
    /// * `expr` - The [`ExprId`] of the condition that must persist throughout the plan.
    pub fn always(&mut self, expr: ExprId) -> ExprId {
        // Optimization: always(True) is simply True.
        if expr == self.empty_and() {
            return expr;
        }
        self.intern(ExprEntryKind::Always, &[expr])
    }

    /// Constructs a `sometime` constraint: the condition must be satisfied at least
    /// once during the trajectory.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Contradiction Folding**: If `expr` is logically `False`, the requirement
    ///    that it must happen "sometime" can never be met. Returns `False`.
    ///
    /// # Parameters
    /// * `expr` - The [`ExprId`] of the goal or condition to be achieved at some point.
    pub fn sometime(&mut self, expr: ExprId) -> ExprId {
        // Optimization: sometime(False) is an impossible goal, thus False.
        if expr == self.empty_or() {
            return expr;
        }
        self.intern(ExprEntryKind::Sometime, &[expr])
    }

    /// Constructs an `at-most-once` constraint: the condition can be true for
    /// at most one contiguous period of time.
    ///
    /// # Design Note
    /// No trivial simplifications are applied here as the validity of this constraint
    /// depends entirely on the state transitions over time, even for constant values.
    ///
    /// # Parameters
    /// * `expr` - The [`ExprId`] of the condition to monitor for state flips.
    pub fn at_most_once(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::AtMostOnce, &[expr])
    }

    /// Constructs a `sometime-after` constraint: if the `first` condition ever holds,
    /// the `second` condition must hold at some point subsequently.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Vacuous Truth**: If the trigger (`first`) is logically `False`, the
    ///    implication is never triggered and thus never violated. Returns `True`.
    ///
    /// # Parameters
    /// * `first` - The triggering condition (the "if").
    /// * `second` - The subsequent condition (the "then").
    pub fn sometime_after(&mut self, first: ExprId, second: ExprId) -> ExprId {
        // Optimization: If 'first' can never happen, the constraint is satisfied by default.
        if first == self.empty_or() {
            return self.empty_and();
        }
        self.intern(ExprEntryKind::SometimeAfter, &[first, second])
    }

    /// Constructs a `sometime-before` constraint: if the `first` condition ever holds,
    /// the `second` condition must have held at least once before that moment.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Vacuous Truth**: If the trigger (`first`) is logically `False`, the
    ///    requirement for a preceding event is never activated. Returns `True`.
    ///
    /// # Parameters
    /// * `first` - The condition that requires a precedent.
    /// * `second` - The condition that must have occurred previously.
    pub fn sometime_before(&mut self, first: ExprId, second: ExprId) -> ExprId {
        // Optimization: If 'first' is never true, no precedent is required.
        if first == self.empty_or() {
            return self.empty_and();
        }
        self.intern(ExprEntryKind::SometimeBefore, &[first, second])
    }

    /// Constructs a `within` constraint: the condition must hold at least once
    /// before the specified time limit.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Tautology Folding**: If `expr` is already `True`, the constraint that it
    ///    must happen within a certain time is already satisfied. Returns `True`.
    /// 2. **Impossible Deadline**: If the `value` (deadline) is significantly negative,
    ///    it is physically impossible to satisfy the condition. Returns `False`.
    ///
    /// # Parameters
    /// * `value` - The maximum time allowed for the expression to become true.
    /// * `expr` - The [`ExprId`] of the condition to monitor.
    pub fn within(&mut self, value: f64, expr: ExprId) -> ExprId {
        // Optimization: If it's already True, the deadline is irrelevant.
        if expr == self.empty_and() {
            return expr;
        }

        // Optimization: A negative deadline is a structural contradiction in PDDL.
        // We use is_neg to allow for near-zero negative values caused by float imprecision.
        if self.is_neg(value) {
            return self.empty_or();
        }

        let duration_node = self.number(value);
        self.intern(ExprEntryKind::Within, &[duration_node, expr])
    }

    /// Constructs an `always-within` constraint: whenever the `first` condition occurs,
    /// the `second` condition must hold within the specified `duration`.
    ///
    /// # Trivial Simplifications
    ///
    /// 1. **Vacuous Truth**: Based on the logic of temporal implication, if the trigger (`first`)
    ///    is logically `False`, the constraint can never be violated. Returns `True`.
    ///
    /// # Parameters
    /// * `duration` - The time window after each occurrence of `first` where `second` must hold.
    /// * `first` - The triggering condition.
    /// * `second` - The condition that must follow.
    pub fn always_within(&mut self, duration: f64, first: ExprId, second: ExprId) -> ExprId {
        // Optimization: If the trigger never happens, the requirement is satisfied by default.
        if first == self.empty_or() {
            return self.empty_and();
        }

        let number_node = self.number(duration);
        self.intern(ExprEntryKind::AlwaysWithin, &[number_node, first, second])
    }

    /// Constructs a `hold-during` constraint: the condition must hold throughout the interval `[start, end]`.
    ///
    /// # Trivial Simplifications
    ///
    /// To optimize the expression graph, the following rules are applied:
    /// 1. **Tautology Folding**: If `expr` is already logically `True`, the constraint is
    ///    always satisfied regardless of the time interval. Returns `True`.
    /// 2. **Impossible Interval**: If `start` is significantly greater than `end`, the interval
    ///    is physically unreachable in a forward-moving timeline. Returns `False`.
    ///
    /// # Parameters
    /// * `start` - The timestamp marking the beginning of the required period.
    /// * `end` - The timestamp marking the end of the required period.
    /// * `expr` - The [`ExprId`] of the condition to monitor.
    pub fn hold_during(&mut self, start: f64, end: f64, expr: ExprId) -> ExprId {
        // Optimization: If the condition is always True, the temporal constraint is redundant.
        if expr == self.empty_and() {
            return expr;
        }

        // Optimization: PDDL trajectories move forward. An inverted interval is a contradiction.
        // We use is_gt to avoid rejecting intervals that are nearly zero due to rounding.
        if self.is_gt(start, end) {
            return self.empty_or();
        }

        let start_node = self.number(start);
        let end_node = self.number(end);
        self.intern(ExprEntryKind::HoldDuring, &[start_node, end_node, expr])
    }

    /// Constructs a `hold-after` constraint: the condition must hold for all timestamps `t >= time`.
    ///
    /// # Trivial Simplifications
    ///
    /// To optimize the expression graph, the following rule is applied:
    /// 1. **Tautology Folding**: If `expr` is already logically `True`, the constraint
    ///    is satisfied for any future state. Returns `True`.
    ///
    /// # Parameters
    /// * `time` - The timestamp after which the condition must remain True until the end of the plan.
    /// * `expr` - The [`ExprId`] of the condition to monitor.
    pub fn hold_after(&mut self, time: f64, expr: ExprId) -> ExprId {
        // Optimization: A condition that is always True satisfies any 'hold-after' requirement.
        if expr == self.empty_and() {
            return expr;
        }

        let time_node = self.number(time);
        self.intern(ExprEntryKind::HoldAfter, &[time_node, expr])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::VariableId;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Objective: Verify that 'always' correctly folds True and interns other expressions.
    /// Input: always(True) and always(variable).
    /// Output: True constant and an Always node respectively.
    #[test]
    fn test_always_normalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));

        let true_val = builder.empty_and();
        assert_eq!(builder.always(true_val), true_val);

        let id = builder.always(var);
        let node = builder.get(id).expect("Node must exist");
        assert!(matches!(node.kind(), ExprEntryKind::Always));
    }

    /// Objective: Verify that 'sometime' correctly folds False and interns other expressions.
    /// Input: sometime(False) and sometime(variable).
    /// Output: False constant and a Sometime node respectively.
    #[test]
    fn test_sometime_normalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));

        let false_val = builder.empty_or();
        assert_eq!(builder.sometime(false_val), false_val);

        let id = builder.sometime(var);
        let node = builder.get(id).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Sometime));
    }

    /// Objective: Ensure 'at_most_once' always interns without folding.
    /// Input: at_most_once(True) and at_most_once(variable).
    /// Output: Distinct AtMostOnce nodes for both (no folding).
    #[test]
    fn test_at_most_once_structure() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();

        let id = builder.at_most_once(var);
        assert!(matches!(
            builder.get(id).unwrap().kind(),
            ExprEntryKind::AtMostOnce
        ));

        let id_true = builder.at_most_once(true_val);
        assert!(matches!(
            builder.get(id_true).unwrap().kind(),
            ExprEntryKind::AtMostOnce
        ));
    }

    /// Objective: Verify vacuous truth folding for 'sometime_after' and 'sometime_before'.
    /// Input: Trigger is False (empty_or).
    /// Output: True constant (empty_and).
    #[test]
    fn test_temporal_implication_vacuous_truth() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let falsy = builder.empty_or();
        let truthy = builder.empty_and();

        assert_eq!(builder.sometime_after(falsy, var), truthy);
        assert_eq!(builder.sometime_before(falsy, var), truthy);
    }

    /// Objective: Test 'within' for tautology folding and negative deadline rejection.
    /// Input: within(10.0, True) and within(-1.0, variable).
    /// Output: True constant and False constant respectively.
    #[test]
    fn test_within_optimizations() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();
        let false_val = builder.empty_or();

        assert_eq!(builder.within(10.0, true_val), true_val);
        assert_eq!(builder.within(-5.0, var), false_val);

        let id = builder.within(10.0, var);
        let node = builder.get(id).unwrap();
        let duration_id = node.children()[0];
        assert!(
            matches!(builder.get(duration_id).unwrap().kind(), ExprEntryKind::Number(n) if *n == 10.0)
        );
    }

    /// Objective: Test 'hold_during' for interval validation and tautology folding.
    /// Input: hold_during(5.0, 10.0, True) and hold_during(10.0, 5.0, variable).
    /// Output: True constant and False constant respectively.
    #[test]
    fn test_hold_during_validation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();
        let false_val = builder.empty_or();

        assert_eq!(builder.hold_during(0.0, 10.0, true_val), true_val);
        assert_eq!(builder.hold_during(10.0, 5.0, var), false_val);

        let id = builder.hold_during(5.0, 10.0, var);
        assert_eq!(builder.get(id).unwrap().children().len(), 3);
    }

    /// Objective: Verify 'hold_after' folding and structure.
    /// Input: hold_after(10.0, True) and hold_after(10.0, variable).
    /// Output: True constant and a HoldAfter node.
    #[test]
    fn test_hold_after_logic() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();

        assert_eq!(builder.hold_after(10.0, true_val), true_val);

        let id = builder.hold_after(10.0, var);
        assert!(matches!(
            builder.get(id).unwrap().kind(),
            ExprEntryKind::HoldAfter
        ));
    }

    /// Objective: Verify 'always_within' vacuous truth.
    /// Input: always_within(5.0, False, variable).
    /// Output: True constant.
    #[test]
    fn test_always_within_vacuous() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.variable(VariableId::from(1));
        let falsy = builder.empty_or();
        let truthy = builder.empty_and();

        assert_eq!(builder.always_within(5.0, falsy, var), truthy);
    }
}
