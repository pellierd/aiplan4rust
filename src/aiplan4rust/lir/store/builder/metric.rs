//! # Optimization Metrics and Structural Constraints
//!
//! This module handles the construction of plan metrics and structural constraints
//! for the planning problem. It defines the optimization objectives (e.g., minimizing
//! costs or time) and physical limits on the plan's structure.
//!
//! ## Design Principles
//!
//! 1. **Semantic Preservation**: Metrics like `minimize` or `maximize` are treated
//!    as top-level intent. Even if they wrap a constant value, they are preserved
//!    in the expression graph to ensure the solver identifies the objective.
//!
//! 2. **Canonical Variables**: Special PDDL variables such as `total-time` and
//!    `total-cost` are interned as unique, childless nodes. This allows the
//!    evaluator to access global plan state registers in $O(1)$ during state expansion.
//!
//! 3. **Physical Validity**: Structural constraints (Serial/Parallel lengths)
//!    leverage epsilon-aware checks ($10^{-9}$) to immediately reject impossible
//!    requirements, such as negative plan lengths.
//!
//! ## Examples
//!
//! ```rust
//! // To minimize total-cost:
//! let cost_var = builder.total_cost();
//! let objective = builder.minimize(cost_var);
//!
//! // To set a maximum serial length of 50 steps:
//! let constraint = builder.length(Some(50.0), None);
//! ```

use crate::aiplan4rust::lang::OptimizationOp;
use crate::aiplan4rust::lir::store::builder::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Constructs a metric expression for plan optimization.
    ///
    /// This node defines the optimization objective. Note that we do not simplify
    /// expressions like `minimize(0)` here, as the metric declaration is a
    /// fundamental part of the problem's structural definition.
    ///
    /// # Arguments
    ///
    /// * `opt` - The [`OptimizationOp`] defining the direction (Minimize/Maximize).
    /// * `expr` - The [`ExprId`] of the functional expression to be optimized.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the newly created Metric node.
    pub fn metric_exp(&mut self, opt: OptimizationOp, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::Metric(opt), &[expr])
    }

    /// Creates a minimization objective for the given expression.
    ///
    /// # Arguments
    ///
    /// * `expr` - The [`ExprId`] representing the value to minimize (e.g., total-cost).
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the Minimize Metric node.
    pub fn minimize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Minimize, expr)
    }

    /// Creates a maximization objective for the given expression.
    ///
    /// # Arguments
    ///
    /// * `expr` - The [`ExprId`] representing the value to maximize (e.g., reward).
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the Maximize Metric node.
    pub fn maximize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Maximize, expr)
    }

    /// Returns the unique identifier for the `total-time` (makespan) variable.
    ///
    /// This represents the duration of the entire plan. Hash-Consing ensures
    /// that this variable is canonical and unique across the expression store.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the global `total-time` variable.
    pub fn total_time(&mut self) -> ExprId {
        self.intern(ExprEntryKind::TotalTime, &[])
    }

    /// Returns the unique identifier for the `total-cost` variable.
    ///
    /// Typically used in domains with action costs to track the cumulative
    /// cost of all operators triggered in the plan.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The ID of the global `total-cost` variable.
    pub fn total_cost(&mut self) -> ExprId {
        self.intern(ExprEntryKind::TotalCost, &[])
    }

    /// Constructs plan length constraints, potentially combining serial and parallel limits.
    ///
    /// # Arguments
    ///
    /// * `serial` - An optional limit for the number of serial steps.
    /// * `parallel` - An optional limit for the number of parallel steps.
    ///
    /// # Returns
    ///
    /// * `ExprId` - A combined Length node, a specific Serial/Parallel node,
    ///   or a `True` constant ([`empty_and`]) if no constraints are provided.
    pub fn length(&mut self, serial: Option<f64>, parallel: Option<f64>) -> ExprId {
        match (serial, parallel) {
            (None, None) => self.empty_and(), // No constraints is vacuously True
            (Some(s), None) => self.serial(s),
            (None, Some(p)) => self.parallel(p),
            (Some(s), Some(p)) => {
                let s_id = self.serial(s);
                let p_id = self.parallel(p);
                self.intern(ExprEntryKind::Length, &[s_id, p_id])
            }
        }
    }

    /// Defines a constraint on the number of serial steps in the plan.
    ///
    /// # Arguments
    ///
    /// * `value` - The maximum allowed serial length.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The Serial constraint node, or a `False` constant ([`empty_or`])
    ///   if the value is negative (impossible constraint).
    pub fn serial(&mut self, value: f64) -> ExprId {
        if self.is_neg(value) {
            return self.empty_or();
        }
        let number = self.number(value);
        self.intern(ExprEntryKind::Serial, &[number])
    }

    /// Defines a constraint on the number of parallel steps in the plan.
    ///
    /// # Arguments
    ///
    /// * `value` - The maximum allowed parallel length.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The Parallel constraint node, or a `False` constant ([`empty_or`])
    ///   if the value is negative (impossible constraint).
    pub fn parallel(&mut self, value: f64) -> ExprId {
        if self.is_neg(value) {
            return self.empty_or();
        }
        let number = self.number(value);
        self.intern(ExprEntryKind::Parallel, &[number])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::store::ExprStore;

    /// Verifies that special PDDL variables (total-time, total-cost) are
    /// properly interned and benefit from Hash-Consing.
    #[test]
    fn test_special_variables_uniqueness() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let t1 = builder.total_time();
        let t2 = builder.total_time();
        let c1 = builder.total_cost();
        let c2 = builder.total_cost();

        assert_eq!(t1, t2, "total-time must be canonical");
        assert_eq!(c1, c2, "total-cost must be canonical");
        assert_ne!(t1, c1, "total-time and total-cost must be distinct");

        // Use fetch to verify existence and retrieve the node
        let node = builder.fetch(t1).expect("total-time node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::TotalTime));
    }

    /// Verifies that metrics preserve their intent even with constant expressions.
    #[test]
    fn test_metrics_preservation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let zero = builder.number(0.0);

        let min_id = builder.minimize(zero);

        // Fetch returns a Result, perfect for tests
        let node = builder
            .fetch(min_id)
            .expect("Metric node should be fetchable");

        if let ExprEntryKind::Metric(opt) = node.kind() {
            assert_eq!(*opt, OptimizationOp::Minimize);
            assert_eq!(node.children()[0], zero);
        } else {
            panic!("Expected Metric node, got {:?}", node.kind());
        }
    }

    /// Tests the 'length' helper for all combinations of Option parameters.
    #[test]
    fn test_length_combinations() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Case: None, None -> True (empty_and)
        assert_eq!(builder.length(None, None), builder.empty_and());

        // Case: Serial only
        let serial_only = builder.length(Some(10.0), None);
        let s_node = builder.fetch(serial_only).unwrap();
        assert!(matches!(s_node.kind(), ExprEntryKind::Serial));

        // Case: Both
        let both = builder.length(Some(10.0), Some(20.0));
        let b_node = builder.fetch(both).expect("Length node should exist");

        assert!(matches!(b_node.kind(), ExprEntryKind::Length));
        assert_eq!(b_node.children().len(), 2);
    }

    /// Verifies that negative plan lengths are rejected using the 1e-9 epsilon.
    /// This test ensures that real contradictions are caught while floating-point
    /// noise near zero is tolerated.
    #[test]
    fn test_length_epsilon_robustness() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let falsy = builder.empty_or();

        // 1. Rejet des vrais négatifs (significatifs)
        assert_eq!(builder.serial(-1.0), falsy);
        assert_eq!(builder.parallel(-1.1e-9), falsy); // Juste au-delà de l'epsilon

        // 2. Tolérance du bruit (micro-négatif)
        let tiny_neg = -1e-12;
        let res_id = builder.serial(tiny_neg);

        assert_ne!(res_id, falsy, "Should tolerate tiny negative as 0.0");

        // 3. Vérification structurelle simplifiée
        let node = builder.fetch(res_id).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Serial));

        // On récupère la valeur numérique stockée
        let num_id = node.children()[0];
        let num_val = builder.fetch(num_id).unwrap();

        if let ExprEntryKind::Number(val) = num_val.kind() {
            // On vérifie que la valeur est dans la zone de tolérance
            assert!(val.into_inner() >= -ExprBuilder::EPSILON);
        } else {
            panic!("Expected a number node");
        }
    }

    /// Verifies that maximize and minimize are distinct and preserve their op.
    #[test]
    fn test_maximize_vs_minimize() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let cost = builder.total_cost();

        let min_id = builder.minimize(cost);
        let max_id = builder.maximize(cost);

        assert_ne!(
            min_id, max_id,
            "Minimize and Maximize must be distinct nodes"
        );

        let node_max = builder.fetch(max_id).unwrap();
        assert!(matches!(
            node_max.kind(),
            ExprEntryKind::Metric(OptimizationOp::Maximize)
        ));
    }

    /// Verifies Hash-Consing idempotence for complex length constraints.
    #[test]
    fn test_length_idempotence() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let l1 = builder.length(Some(10.0), Some(20.0));
        let l2 = builder.length(Some(10.0), Some(20.0));

        assert_eq!(
            l1, l2,
            "Redundant length constraints must return the same ExprId"
        );
    }

    /// Quick check that parallel also uses epsilon robustness.
    #[test]
    fn test_parallel_epsilon() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let falsy = builder.empty_or();

        assert_eq!(builder.parallel(-1.0), falsy);
        assert_ne!(builder.parallel(-1e-12), falsy);
    }
}
