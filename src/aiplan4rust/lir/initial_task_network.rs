//! This module defines the `InitialTaskNetwork` struct, representing
//! an initial task network in a hierarchical task network syntax domain.
//!
//! The `InitialTaskNetwork` consists of a list of typed parameters and
//! a lifted task network describing the tasks and their relationships.

use crate::aiplan4rust::lang::{TypeID, TypedList, VariableID};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::aiplan4rust::lir::{renderers, LiftedTaskNetwork};

/// Represents the initial task network, containing parameters and a lifted task network.
///
/// This struct encapsulates the starting point of a hierarchical task network
/// with its parameters and the task network that specifies the initial tasks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InitialTaskNetwork {
    /// The typed parameters of the initial task network.
    parameters: TypedList<VariableID, TypeID>,

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
    pub fn new(parameters: TypedList<VariableID, TypeID>, task_network: LiftedTaskNetwork) -> Self {
        Self { parameters, task_network }
    }

    /// Returns an immutable reference to the parameters.
    pub fn parameters(&self) -> &TypedList<VariableID, TypeID> {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableID, TypeID> {
        &mut self.parameters
    }

    /// Sets the parameters to a new `TypedList`.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableID, TypeID>) {
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


impl Display for InitialTaskNetwork {
    /// Formats the initial task network for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_initial_task_network(f, self)
    }
}
