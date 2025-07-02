use serde::{Deserialize, Serialize};

use crate::aiplan4rust::lir::expr::Expr;

/// Represents a network of tasks along with their ordering and logical constraints.
///
/// A `TaskNetwork` contains:
/// - `tasks`: an expression representing the set or list of tasks.
/// - `ordering_constraints`: an expression representing constraints on the order in which tasks should be performed.
/// - `logical_constraints`: an expression representing additional logical constraints on the tasks.
///
/// This structure is used to model task networks in planning domains,
/// where tasks might have partial or total ordering and other logical dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TaskNetwork {
    tasks: Expr,
    ordering_constraints: Expr,
    logical_constraints: Expr,
}

impl TaskNetwork {
    /// Creates a new `TaskNetwork` with the given tasks, ordering constraints, and logical constraints.
    ///
    /// # Parameters
    ///
    /// - `tasks`: Expression representing the tasks involved in the network.
    /// - `ordering_constraints`: Expression representing ordering constraints between tasks.
    /// - `logical_constraints`: Expression representing logical constraints among tasks.
    ///
    /// # Returns
    ///
    /// A new instance of `TaskNetwork`.
    pub fn new(tasks: Expr, ordering_constraints: Expr, logical_constraints: Expr) -> Self {
        Self {
            tasks,
            ordering_constraints,
            logical_constraints,
        }
    }

    /// Returns a reference to the tasks expression.
    pub fn tasks(&self) -> &Expr {
        &self.tasks
    }

    /// Returns a mutable reference to the tasks expression.
    pub fn tasks_mut(&mut self) -> &mut Expr {
        &mut self.tasks
    }

    /// Sets the tasks expression.
    ///
    /// # Parameters
    ///
    /// - `tasks`: The new tasks expression to set.
    pub fn set_tasks(&mut self, tasks: Expr) {
        self.tasks = tasks;
    }

    /// Returns a reference to the ordering constraints expression.
    pub fn ordering_constraints(&self) -> &Expr {
        &self.ordering_constraints
    }

    /// Returns a mutable reference to the ordering constraints expression.
    pub fn ordering_constraints_mut(&mut self) -> &mut Expr {
        &mut self.ordering_constraints
    }

    /// Sets the ordering constraints expression.
    ///
    /// # Parameters
    ///
    /// - `ordering_constraints`: The new ordering constraints expression to set.
    pub fn set_ordering_constraints(&mut self, ordering_constraints: Expr) {
        self.ordering_constraints = ordering_constraints;
    }

    /// Returns a reference to the logical constraints expression.
    pub fn logical_constraints(&self) -> &Expr {
        &self.logical_constraints
    }

    /// Returns a mutable reference to the logical constraints expression.
    pub fn logical_constraints_mut(&mut self) -> &mut Expr {
        &mut self.logical_constraints
    }

    /// Sets the logical constraints expression.
    ///
    /// # Parameters
    ///
    /// - `logical_constraints`: The new logical constraints expression to set.
    pub fn set_logical_constraints(&mut self, logical_constraints: Expr) {
        self.logical_constraints = logical_constraints;
    }
}
