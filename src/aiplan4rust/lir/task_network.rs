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
//! use crate::aiplan4rust::lir::expr::Expr;
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
//! The expressions used are built from the [`Expr`] representation, which supports
//! logical combinations, references to task calls, and symbolic constructs parsed from ASTs.

use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::lang::{StringID, TaskSkeletonID};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::problem::normalize;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};
use crate::aiplan4rust::tree::NodeId;

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
    tasks: Expr,
    ordering_constraints: Expr,
    logical_constraints: Expr,
    is_declared_total_ordered: bool,
    task_labels: Vec<StringID>,
    task_def: Vec<TaskSkeletonID>,
    task_nodes: Vec<NodeId>,
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
        tasks: Expr,
        ordering_constraints: Expr,
        logical_constraints: Expr,
        is_declared_total_ordered: bool,
        task_labels: Vec<StringID>,
        task_def: Vec<TaskSkeletonID>,
        task_nodes: Vec<NodeId>,
    ) -> Self {
        Self {
            tasks,
            ordering_constraints,
            logical_constraints,
            is_declared_total_ordered,
            task_labels,
            task_def,
            task_nodes

        }
    }

    /// Returns an immutable reference to the tasks expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the tasks.
    pub fn tasks(&self) -> &Expr {
        &self.tasks
    }

    /// Returns a mutable reference to the tasks expression.
    ///
    /// This allows modifying the tasks contained in the network.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`Expr`] representing the tasks.
    pub fn tasks_mut(&mut self) -> &mut Expr {
        &mut self.tasks
    }

    /// Sets the tasks expression.
    ///
    /// # Parameters
    ///
    /// - `tasks`: The new [`Expr`] representing the tasks to replace the current one.
    pub fn set_tasks(&mut self, tasks: Expr) {
        self.tasks = tasks;
    }

    /// Returns an immutable reference to the ordering constraints expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the ordering constraints.
    pub fn ordering_constraints(&self) -> &Expr {
        &self.ordering_constraints
    }

    /// Returns a mutable reference to the ordering constraints expression.
    ///
    /// This allows modifying the ordering constraints.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`Expr`] representing the ordering constraints.
    pub fn ordering_constraints_mut(&mut self) -> &mut Expr {
        &mut self.ordering_constraints
    }

    /// Sets the ordering constraints expression.
    ///
    /// # Parameters
    ///
    /// - `ordering_constraints`: The new [`Expr`] representing the ordering constraints.
    pub fn set_ordering_constraints(&mut self, ordering_constraints: Expr) {
        self.ordering_constraints = ordering_constraints;
    }

    /// Returns an immutable reference to the logical constraints expression.
    ///
    /// # Returns
    ///
    /// A reference to the [`Expr`] representing the logical constraints.
    pub fn logical_constraints(&self) -> &Expr {
        &self.logical_constraints
    }

    /// Returns a mutable reference to the logical constraints expression.
    ///
    /// This allows modifying the logical constraints.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`Expr`] representing the logical constraints.
    pub fn logical_constraints_mut(&mut self) -> &mut Expr {
        &mut self.logical_constraints
    }

    /// Sets the logical constraints expression.
    ///
    /// # Parameters
    ///
    /// - `logical_constraints`: The new [`Expr`] representing the logical constraints.
    pub fn set_logical_constraints(&mut self, logical_constraints: Expr) {
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

    /// Normalizes the task network in-place by normalizing its logical constraints.
    ///
    /// This ensures that the expressions within the task network are in canonical form.
    /// Currently, only `logical_constraints` require normalization; `tasks` and
    /// `ordering_constraints` are structurally fixed and do not need normalization.
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if normalization fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut network = TaskNetwork::default();
    /// network.normalize()?;
    /// ```
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_task_network(self)?)
    }
}

impl Display for TaskNetwork {
    /// Formats the `TaskNetwork` as a human-readable string.
    ///
    /// This implementation uses the default renderer to display the tasks,
    /// ordering constraints, and logical constraints in a readable form.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fmt::Write;
    /// # let network: TaskNetwork = todo!();
    /// let mut s = String::new();
    /// write!(&mut s, "{}", network).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        renderers::default::render_task_network(f, self)
    }
}

impl LiftedSyntaxDisplay for TaskNetwork {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::task_network::render_task_network(f, self, ctx)
    }
}
