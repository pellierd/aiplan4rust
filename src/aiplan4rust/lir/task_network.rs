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
//! use aiplan4rust::lir::task_network::TaskNetwork;
//! use aiplan4rust::lir::expr::Expr;
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

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

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
}

#[allow(dead_code)]
impl TaskNetwork {
    /// Creates a new `TaskNetwork` with the specified tasks and constraints.
    ///
    /// # Parameters
    ///
    /// - `tasks`: An [`Expr`] representing the tasks included in the network.
    /// - `ordering_constraints`: An [`Expr`] specifying the ordering between tasks.
    /// - `logical_constraints`: An [`Expr`] specifying additional logical constraints among the tasks.
    ///
    /// # Returns
    ///
    /// A new [`TaskNetwork`] instance.
    pub fn new(tasks: Expr, ordering_constraints: Expr, logical_constraints: Expr) -> Self {
        Self {
            tasks,
            ordering_constraints,
            logical_constraints,
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
}

/// Attempts to construct a [`TaskNetwork`] from a given [`SyntaxSubtree`]
/// referencing an AST node and its associated syntax tree.
///
/// # Expected Structure
///
/// The node should contain children matching one or more of the following:
/// - `PartiallyOrderedSubtaskDef` or `OrderedSubtaskDef`: holds the task definitions.
/// - `TaskOrderingConstraintDef`: holds ordering constraints.
/// - `TaskLogicalConstraintDef`: holds logical constraints.
///
/// # Returns
///
/// - `Ok(TaskNetwork)` if all components are successfully parsed.
/// - `Err(AiplanError)` if the syntax structure is unexpected or a subcomponent fails.
///
/// # Example
///
/// ```rust,ignore
/// let task_network = TaskNetwork::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for TaskNetwork {
    type Error = LirError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        let children = node.children();

        let mut tasks = Expr::empty_and();
        let mut ordering = Expr::empty_and();
        let mut constraints = Expr::empty_and();

        for &child_id in children {
            let child_node = ast.try_node(child_id)?;
            match child_node.kind() {
                AstKind::PartiallyOrderedSubtaskDef | AstKind::OrderedSubtaskDef => {
                    let tasks_node_id = child_node.try_child(0)?;
                    let tasks_node = ast.try_node(tasks_node_id)?;
                    tasks = Expr::try_from(&SyntaxSubtree::new(tasks_node, ast))?;
                }
                AstKind::TaskOrderingConstraintDef => {
                    let ordering_node_id = child_node.try_child(0)?;
                    let ordering_node = ast.try_node(ordering_node_id)?;
                    ordering = Expr::try_from(&SyntaxSubtree::new(ordering_node, ast))?;
                }
                AstKind::TaskLogicalConstraintDef => {
                    let logical_node_id = child_node.try_child(0)?;
                    let logical_node = ast.try_node(logical_node_id)?;
                    constraints = Expr::try_from(&SyntaxSubtree::new(logical_node, ast))?;
                }
                _ => {
                    return Err(LirError::task_network_ast_kind_error(child_node.kind()));
                }
            }
        }

        Ok(TaskNetwork::new(tasks, ordering, constraints))
    }
}


impl Display for TaskNetwork {
    /// Formats the `TaskNetwork` as a human-readable string.
    ///
    /// This representation displays the tasks, ordering constraints, and logical constraints.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tasks: {}", self.tasks)?;
        writeln!(f, "Ordering: {}", self.ordering_constraints)?;
        writeln!(f, "Constraints: {}", self.logical_constraints)
    }
}

impl InternerDisplay for TaskNetwork {
    /// Formats the `TaskNetwork` using the provided [`StringInterner`] to resolve identifiers.
    ///
    /// This representation is useful for reconstructing meaningful names in debug output.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> std::fmt::Result {
        write!(f, "TASKS\n{}", self.tasks.to_string_with_interner(interner))?;
        write!(f, "ORDERING\n{}", self.ordering_constraints.to_string_with_interner(interner))?;
        write!(f, "CONSTRAINTS\n{}", self.logical_constraints.to_string_with_interner(interner))
    }
}

impl SyntaxDisplay for TaskNetwork {
    /// Formats the `TaskNetwork` in a syntax-oriented form using the provided [`StringInterner`].
    ///
    /// This representation can be used to regenerate source-like output.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> std::fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
