//! This module defines the `InitialTaskNetwork` struct, representing
//! an initial task network in a hierarchical task network syntax domain.
//!
//! The `InitialTaskNetwork` consists of a list of typed parameters and
//! a lifted task network describing the tasks and their relationships.

use crate::aiplan4rust::lang::{TaskLabelSymbolId, TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::problem::SymbolRegistry;
use crate::aiplan4rust::lir::problem::TaskNetwork;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Represents the initial task network, containing parameters and a lifted task network.
///
/// This struct encapsulates the starting point of a hierarchical task network
/// with its parameters and the task network that specifies the initial tasks.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InitialTaskNetwork {
    /// The typed parameters of the initial task network.
    parameters: TypedList<VariableId, TypeId>,

    /// The lifted task network describing the initial tasks.
    task_network: TaskNetwork,

    variable_symbols: SymbolRegistry<VariableId>,
    task_label_symbols: SymbolRegistry<TaskLabelSymbolId>,
}

impl InitialTaskNetwork {
    /// Creates a new `InitialTaskNetwork` with given parameters and task network.
    ///
    /// # Parameters
    /// - `parameters`: The typed parameters for the initial task network.
    /// - `task_network`: The lifted task network representing tasks.
    ///
    /// # Returns
    /// A new instance of `InitialTaskNetwork`.
    pub fn new(parameters: TypedList<VariableId, TypeId>, task_network: TaskNetwork) -> Self {
        Self {
            parameters,
            task_network,
            variable_symbols: SymbolRegistry::new(),
            task_label_symbols: SymbolRegistry::new(),
        }
    }

    /// Injecte le registre des symboles de variables.
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    /// Injecte le registre des étiquettes de tâches.
    pub fn with_task_label_symbols(mut self, symbols: SymbolRegistry<TaskLabelSymbolId>) -> Self {
        self.task_label_symbols = symbols;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.task_network.is_empty()
    }

    /// Returns an immutable reference to the parameters.
    pub fn parameters(&self) -> &TypedList<VariableId, TypeId> {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableId, TypeId> {
        &mut self.parameters
    }

    /// Sets the parameters to a new `TypedList`.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableId, TypeId>) {
        self.parameters = parameters;
    }

    /// Returns an immutable reference to the task network.
    pub fn task_network(&self) -> &TaskNetwork {
        &self.task_network
    }

    /// Returns a mutable reference to the task network.
    pub fn task_network_mut(&mut self) -> &mut TaskNetwork {
        &mut self.task_network
    }

    /// Sets the task network to a new `LiftedTaskNetwork`.
    pub fn set_task_network(&mut self, task_network: TaskNetwork) {
        self.task_network = task_network;
    }

    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> {
        &self.variable_symbols
    }

    pub fn task_label_symbols(&self) -> &SymbolRegistry<TaskLabelSymbolId> {
        &self.task_label_symbols
    }

    // --- Accesseurs Mutables (Mutators) ---

    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> {
        &mut self.variable_symbols
    }

    pub fn task_label_symbols_mut(&mut self) -> &mut SymbolRegistry<TaskLabelSymbolId> {
        &mut self.task_label_symbols
    }
}

impl LiftedSyntaxDisplay for InitialTaskNetwork {
    /// Délègue le rendu PDDL/HDDL au module spécialisé.
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::initial_task_network::render(f, self, ctx)
    }
}

impl LiftedDebugDisplay for InitialTaskNetwork {
    /// Délègue le rendu structurel (arbre/debug) au module spécialisé.
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::initial_task_network::render(f, self, ctx)
    }
}
