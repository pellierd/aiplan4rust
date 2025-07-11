//! This module defines the `InitialTaskNetwork` struct, representing
//! an initial task network in a hierarchical task network planning domain.
//!
//! The `InitialTaskNetwork` consists of a list of typed parameters and
//! a lifted task network describing the tasks and their relationships.

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LiftedTaskNetwork;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

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

impl FromAst for InitialTaskNetwork {
    /// Parses an `InitialTaskNetwork` from an AST node.
    ///
    /// # Parameters
    /// - `node`: The AST node representing the initial task network.
    /// - `ast`: The arena containing all AST nodes.
    ///
    /// # Returns
    /// - `Ok(InitialTaskNetwork)` if parsing succeeds.
    /// - `Err(ParserInternalError)` if parsing fails.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let mut child_index = 0;

        // Try to parse parameters if present, otherwise use empty parameters
        let parameters_def_id = node.try_child(child_index)?;
        let parameters_def_node = ast.try_node(parameters_def_id)?;
        let parameters = match parameters_def_node.kind() {
            AstKind::ParametersDef => {
                let param_node_id = parameters_def_node.try_child(0)?;
                let param_node = ast.try_node(param_node_id)?;
                child_index += 1;
                TypedList::from_ast(param_node, ast)?
            }
            _ => TypedList::empty(),
        };

        // Parse the lifted task network
        let tw_node_id = node.try_child(child_index)?;
        let tw_node = ast.try_node(tw_node_id)?;
        let tw = LiftedTaskNetwork::from_ast(tw_node, ast)?;

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

impl DisplayWithInterner for InitialTaskNetwork {
    /// Formats the initial task network using a string interner for symbol resolution.
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        writeln!(f, "Parameters: {}", self.parameters.to_string_with_interner(interner))?;
        writeln!(f, "Task Network: {}", self.task_network.to_string_with_interner(interner))
    }
}

impl PlanningSyntaxDisplay for InitialTaskNetwork {
    /// Formats the initial task network syntax using a string interner.
    fn fmt_planning_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        // Write the indentation prefix
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with(f, interner)
    }
}
