//! This module defines the `InitialTaskNetwork` struct, representing
//! an initial task network in a hierarchical task network syntax domain.
//!
//! The `InitialTaskNetwork` consists of a list of typed parameters and
//! a lifted task network describing the tasks and their relationships.

use std::collections::HashMap;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{RemapTypes, StringID, Type, TypeID, TypedList};
use crate::aiplan4rust::lir::problem::{normalize, renderers, LiftedTaskNetwork};
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::aiplan4rust::lir::error::LirError;

/// Represents the initial task network, containing parameters and a lifted task network.
///
/// This struct encapsulates the starting point of a hierarchical task network
/// with its parameters and the task network that specifies the initial tasks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct InitialTaskNetwork {
    /// The typed parameters of the initial task network.
    parameters: TypedList<TypeID>,

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
    pub fn new(parameters: TypedList<TypeID>, task_network: LiftedTaskNetwork) -> Self {
        Self { parameters, task_network }
    }

    /// Returns an immutable reference to the parameters.
    pub fn parameters(&self) -> &TypedList<TypeID> {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    pub fn parameters_mut(&mut self) -> &mut TypedList<TypeID> {
        &mut self.parameters
    }

    /// Sets the parameters to a new `TypedList`.
    pub fn set_parameters(&mut self, parameters: TypedList<TypeID>) {
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

    /// Normalizes the underlying `task_network` of this `InitialTaskNetwork`.
    ///
    /// Only the `logical_constraints` of the `task_network` are normalized,
    /// since `tasks` and `ordering_constraints` are always in a fixed canonical form.
    ///
    /// # Errors
    ///
    /// Returns an `ExprError` if normalization of the `logical_constraints` fails.
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_initial_task_network(self)?)
    }
}

/*impl RemapTypes for InitialTaskNetwork {
    /// Remaps union types (`Type::Either`) in the network's parameters.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during remapping (e.g., a union type has no corresponding mapping).
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        self.parameters.remap_types(map)?;
        Ok(())
    }
}*/

impl Display for InitialTaskNetwork {
    /// Formats the initial task network for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_initial_task_network(f, self)
    }
}

/*impl InternerDisplay for InitialTaskNetwork {
    /// Formats the initial task network using a string interner for symbol resolution.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        renderers::interner::render_initial_task_network(f, self, interner)
    }
}*/

impl SyntaxInternerDisplay for InitialTaskNetwork {
    /// Formats the initial task network syntax using a string interner.
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        renderers::syntax::render_initial_task_network(f, self, interner, indent)
    }
}
