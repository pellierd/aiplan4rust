//! Task Network Representation
//!
//! This module defines the [`TaskNetwork`] struct used in hierarchical syntax.
//! A task network specifies a partially ordered set of tasks to execute, and may
//! include additional ordering and logical constraints that influence execution.
//!
//! This structure is fundamental to HDDL-like syntax languages, where a method
//! decomposes a high-level task into a network of subtasks.
//!
//! # Structure
//!
//! A `TaskNetwork` contains:
//!
//! - `tasks`: an [`Expr`] representing the subtask set (possibly partially ordered).
//! - `ordering_constraints`: an [`Expr`] describing the order relationships between tasks.
//! - `logical_constraints`: an [`Expr`] encoding additional logical conditions on the task execution.
//!
//! # Construction Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::task_network::TaskNetwork;
//! use crate::aiplan4rust::lir::logic::Expr;
//!
//! let network = TaskNetwork::new(
//!     Expr::empty_and(),
//!     Expr::empty_and(),
//!     Expr::empty_and(),
//! );
//! ```
//!
//! # Usage
//! Task networks are commonly used inside method definitions to describe how
//! an abstract task is decomposed into a set of executable or further abstract tasks.
//!
//! The logic used are built from the [`Expr`] representation, which supports
//! logical combinations, references to task calls, and symbolic constructs parsed from ASTs.

use crate::aiplan4rust::lang::TaskSkeletonId;
use crate::aiplan4rust::lir::store::expr::ExprId;
use crate::aiplan4rust::lir::store::renderers;
use crate::aiplan4rust::lir::store::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext,
};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Represents a network of tasks along with their ordering and logical constraints.
///
/// A `TaskNetwork` contains:
/// - `tasks`: an expression representing the set or list of tasks.
/// - `ordering_constraints`: an expression representing constraints on the order in which tasks should be performed.
/// - `logical_constraints`: an expression representing additional logical constraints on the tasks.
///
/// This structure is used to model task networks in syntax domains,
/// where tasks might have partial or total ordering and other logical dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TaskNetwork {
    tasks: ExprId,
    ordering_constraints: ExprId,
    logical_constraints: ExprId,
    is_declared_total_ordered: bool,
    task_def: Vec<TaskSkeletonId>,
    task_nodes: Vec<ExprId>,
}

#[allow(dead_code)]
impl TaskNetwork {
    /// Creates a new `TaskNetwork` with the specified tasks, constraints, and ordering declaration.
    ///
    /// # Parameters
    ///
    /// - `tasks`: An [`Expr`] representing the tasks included in the network.
    /// - `ordering_constraints`: An [`Expr`] specifying ordering relationships between tasks.
    /// - `logical_constraints`: An [`Expr`] specifying additional logical constraints among the tasks.
    /// - `is_declared_total_ordered`: A `bool` indicating whether the task network is explicitly declared as totally ordered (`true`) or allows partial/unordered execution (`false`).
    ///
    /// # Returns
    ///
    /// A new [`TaskNetwork`] instance with the provided tasks, constraints, and ordering declaration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let tn = TaskNetwork::new(
    ///     tasks_expr,
    ///     ordering_constraints_expr,
    ///     logical_constraints_expr,
    ///     true // declared as totally ordered
    /// );
    /// ```
    pub fn new(
        tasks: ExprId,
        ordering_constraints: ExprId,
        logical_constraints: ExprId,
        is_declared_total_ordered: bool,
        task_def: Vec<TaskSkeletonId>,
        task_nodes: Vec<ExprId>,
    ) -> Self {
        Self {
            tasks,
            ordering_constraints,
            logical_constraints,
            is_declared_total_ordered,
            task_def,
            task_nodes,
        }
    }

    /// Returns an immutable reference to the tasks expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the tasks.
    pub fn tasks(&self) -> ExprId {
        self.tasks
    }

    /// Sets the tasks expression.
    ///
    /// # Parameters
    ///
    /// - `tasks`: The new [`Expr`] representing the tasks to replace the current one.
    pub fn set_tasks(&mut self, tasks: ExprId) {
        self.tasks = tasks;
    }

    /// Returns an immutable reference to the ordering constraints expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the ordering constraints.
    pub fn ordering_constraints(&self) -> ExprId {
        self.ordering_constraints
    }

    /// Sets the ordering constraints expression.
    ///
    /// # Parameters
    ///
    /// - `ordering_constraints`: The new [`Expr`] representing the ordering constraints.
    pub fn set_ordering_constraints(&mut self, ordering_constraints: ExprId) {
        self.ordering_constraints = ordering_constraints;
    }

    /// Returns an immutable reference to the logical constraints expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the logical constraints.
    pub fn logical_constraints(&self) -> ExprId {
        self.logical_constraints
    }

    /// Sets the logical constraints expression.
    ///
    /// # Parameters
    ///
    /// - `logical_constraints`: The new [`Expr`] representing the logical constraints.
    pub fn set_logical_constraints(&mut self, logical_constraints: ExprId) {
        self.logical_constraints = logical_constraints;
    }

    /// Returns `true` if the task network was explicitly declared as totally ordered.
    ///
    /// # Returns
    ///
    /// `true` if the network is declared as totally ordered; `false` otherwise.
    pub fn is_declared_total_ordered(&self) -> bool {
        self.is_declared_total_ordered
    }

    /// Sets whether the task network should be considered explicitly totally ordered.
    ///
    /// # Parameters
    ///
    /// - `value`: A boolean indicating the new ordering declaration (`true` = total order, `false` = partial/unordered).
    pub fn set_declared_total_ordered(&mut self, value: bool) {
        self.is_declared_total_ordered = value;
    }

    pub fn is_empty(&self) -> bool {
        self.task_def.is_empty()
    }
}

impl LiftedSyntaxDisplay for TaskNetwork {
    /// Rendu PDDL/HDDL (le "beau" texte pour l'export ou l'utilisateur).
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::task_network::render(f, self, ctx)
    }
}

impl LiftedDebugDisplay for TaskNetwork {
    /// Rendu structurel (l'arbre technique avec IDs et structure interne).
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::task_network::render(f, self, ctx)
    }
}
