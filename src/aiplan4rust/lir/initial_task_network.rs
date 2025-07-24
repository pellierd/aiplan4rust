//! This module defines the `InitialTaskNetwork` struct, representing
//! an initial task network in a hierarchical task network planning domain.
//!
//! The `InitialTaskNetwork` consists of a list of typed parameters and
//! a lifted task network describing the tasks and their relationships.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LiftedTaskNetwork;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::core::arena::ArenaNode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Represents the initial task network, containing parameters and a lifted task network.
///
/// This struct encapsulates the starting point of a hierarchical task network
/// with its parameters and the task network that specifies the initial tasks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InitialTaskNetwork {
    /// The typed parameters of the initial task network.
    parameters: TypedList,

    /// The lifted task network describing the initial tasks.
    task_network: LiftedTaskNetwork,
}

#[allow(dead_code)]
impl InitialTaskNetwork {
    /// Creates a new `InitialTaskNetwork` with given parameters and task network.
    ///
    /// # Parameters
    /// - `parameters`: The typed parameters for the initial task network.
    /// - `task_network`: The lifted task network representing tasks.
    ///
    /// # Returns
    /// A new instance of `InitialTaskNetwork`.
    pub fn new(parameters: TypedList, task_network: LiftedTaskNetwork) -> Self {
        Self { parameters, task_network }
    }

    /// Returns an immutable reference to the parameters.
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }

    /// Sets the parameters to a new `TypedList`.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.parameters = parameters;
    }

    /// Returns an immutable reference to the task network.
    pub fn task_network(&self) -> &LiftedTaskNetwork {
        &self.task_network
    }

    /// Returns a mutable reference to the task network.
    pub fn task_network_mut(&mut self) -> &mut LiftedTaskNetwork {
        &mut self.task_network
    }

    /// Sets the task network to a new `LiftedTaskNetwork`.
    pub fn set_task_network(&mut self, task_network: LiftedTaskNetwork) {
        self.task_network = task_network;
    }
}

/// Attempts to construct an [`InitialTaskNetwork`] from a given [`SyntaxSubtree`]
/// referencing an AST node and its syntax tree.
///
/// # Expected Structure
///
/// The AST node can contain:
/// - Optionally, a `ParametersDef` node as the first child. If found, it is parsed as a `TypedList`.
/// - A `LiftedTaskNetwork` node as the next child.
///
/// # Returns
///
/// - `Ok(InitialTaskNetwork)` if parsing succeeds.
/// - `Err(AiplanError)` if the AST structure is invalid or parsing fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let initial_tn = InitialTaskNetwork::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for InitialTaskNetwork {
    type Error = LirError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        let mut child_index = 0;

        // Try to parse parameters if present, otherwise use empty parameters
        let parameters = if let Ok(parameters_def_id) = node.try_child(child_index) {
            let parameters_def_node = ast.try_node(parameters_def_id)?;
            if parameters_def_node.kind() == AstKind::ParametersDef {
                let param_node_id = parameters_def_node.try_child(0)?;
                let param_node = ast.try_node(param_node_id)?;
                child_index += 1;
                TypedList::try_from(&SyntaxSubtree::new(param_node, ast))?
            } else {
                TypedList::empty()
            }
        } else {
            TypedList::empty()
        };

        // Parse the lifted task network
        let tw_node_id = node.try_child(child_index)?;
        let tw_node = ast.try_node(tw_node_id)?;
        let tw = LiftedTaskNetwork::try_from(&SyntaxSubtree::new(tw_node, ast))?;

        Ok(InitialTaskNetwork::new(parameters, tw))
    }
}


impl Display for InitialTaskNetwork {
    /// Formats the initial task network for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Parameters: {}", self.parameters)?;
        writeln!(f, "Task Network: {}", self.task_network)
    }
}

impl InternerDisplay for InitialTaskNetwork {
    /// Formats the initial task network using a string interner for symbol resolution.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        writeln!(f, "Parameters: {}", self.parameters.to_string_with_interner(interner))?;
        writeln!(f, "Task Network: {}", self.task_network.to_string_with_interner(interner))
    }
}

impl SyntaxDisplay for InitialTaskNetwork {
    /// Formats the initial task network syntax using a string interner.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        // Write the indentation prefix
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
