use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LiftedTaskNetwork;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InitialTaskNetwork {
    parameters: TypedList,
    task_network: LiftedTaskNetwork,
}

#[allow(dead_code)]
impl InitialTaskNetwork {
    /// Crée un nouveau `InitialTaskNetwork`.
    pub fn new(parameters: TypedList, task_network: LiftedTaskNetwork) -> Self {
        Self { parameters, task_network }
    }

    /// Retourne une référence immuable vers la liste des paramètres.
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Retourne une référence mutable vers la liste des paramètres.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }

    /// Remplace la liste des paramètres.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.parameters = parameters;
    }

    /// Retourne une référence immuable vers le réseau de tâches.
    pub fn task_network(&self) -> &LiftedTaskNetwork {
        &self.task_network
    }

    /// Retourne une référence mutable vers le réseau de tâches.
    pub fn task_network_mut(&mut self) -> &mut LiftedTaskNetwork {
        &mut self.task_network
    }

    /// Remplace le réseau de tâches.
    pub fn set_task_network(&mut self, task_network: LiftedTaskNetwork) {
        self.task_network = task_network;
    }
}

impl FromAst for InitialTaskNetwork {
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let mut child_index = 0;

        let param_node_id = node.try_child(child_index)?;
        let param_node = ast.try_node(param_node_id)?;
        let parameters = match param_node.kind() {
            AstKind::TypedList => {
                child_index += 1;
                TypedList::from_ast(param_node, ast)?
            }
            _ => TypedList::empty(),
        };

        let tw_node_id = node.try_child(child_index)?;
        let tw_node = ast.try_node(tw_node_id)?;
        let tw = LiftedTaskNetwork::from_ast(tw_node, ast)?;

        Ok(InitialTaskNetwork::new(parameters, tw))
    }
}

impl Display for InitialTaskNetwork {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Parameters: {}", self.parameters)?;
        writeln!(f, "Task Network: {}", self.task_network)
    }
}

impl DisplayWithInterner for InitialTaskNetwork {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        writeln!(f, "Parameters: {}", self.parameters.to_string_with_interner(interner))?;
        writeln!(f, "Task Network: {}", self.task_network.to_string_with_interner(interner))
    }
}

impl DisplaySyntax for InitialTaskNetwork {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
