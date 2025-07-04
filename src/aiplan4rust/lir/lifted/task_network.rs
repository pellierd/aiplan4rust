use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

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

#[allow(dead_code)]
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

impl FromAst for TaskNetwork {
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let children = node.children();

        // Initialise avec des valeurs par défaut
        let mut tasks = Expr::empty_and();
        let mut ordering = Expr::empty_and();
        let mut constraints = Expr::empty_and();

        for child_id in children {
            let child_node = ast.try_node(*child_id)?;
            match child_node.kind() {
                AstKind::PartiallyOrderedSubtaskDef
                | AstKind::OrderedSubtaskDef => {
                    let tasks_node_id = child_node.try_child(0)?;
                    let tasks_node = ast.try_node(tasks_node_id)?;
                    tasks = Expr::from_ast(tasks_node, ast)?;
                }
                AstKind::TaskOrderingConstraintDef => {
                    let ordering_node_id = child_node.try_child(0)?;
                    let ordering_node = ast.try_node(ordering_node_id)?;
                    ordering = Expr::from_ast(ordering_node, ast)?;
                }
                AstKind::TaskLogicalConstraintDef => {
                    let logical_node_id = child_node.try_child(0)?;
                    let logical_node = ast.try_node(logical_node_id)?;
                    constraints = Expr::from_ast(logical_node, ast)?;
                }
                _ => {
                    return Err(ParserInternalError::new(format!(
                        "Unexpected node kind in TaskNetwork: {}",
                        child_node.kind(),
                    )));
                }
            }
        }

        Ok(TaskNetwork::new(
            tasks,
            ordering,
            constraints,
        ))
    }
}

impl Display for TaskNetwork {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tasks: {}", self.tasks)?;
        writeln!(f, "Ordering: {}", self.ordering_constraints)?;
        writeln!(f, "Constraints: {}", self.logical_constraints)
    }
}

impl DisplayWithInterner for TaskNetwork {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> std::fmt::Result {
        writeln!(f, "Tasks: {:?}", self.tasks.to_string_with_interner(interner))?;
        writeln!(f, "Ordering: {}", self.ordering_constraints.to_string_with_interner(interner))?;
        writeln!(f, "Constraints: {}", self.logical_constraints.to_string_with_interner(interner))
    }
}

impl DisplaySyntax for TaskNetwork {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> std::fmt::Result {
        self.fmt_with(f, interner)
    }
}
