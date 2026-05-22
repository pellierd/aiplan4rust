//! This module defines the `Method` struct, representing a lifted method in a hierarchical task network (HTN) syntax domain.
//!
//! A `Method` describes how a complex task can be decomposed into subtasks under certain preconditions,
//! encapsulating the method's name, parameters, the task it refines, its preconditions, and the resulting task network.
//!
//! This is a tree abstraction for expressing domain methods in HTN syntax.
//!
//! # Overview
//! - `header`: The method’s name and typed parameters (via `NamedTypedList`).
//! - `task`: The task expression that the method refines.
//! - `precondition`: Preconditions required for the method to apply (defaults to empty `Or`).
//! - `task_network`: The lifted task network decomposing the task.
//!
//! # Example
//! ```rust
//! # use crate::aiplan4rust::lang::{Ident, TypedList};
//! # use crate::aiplan4rust::lir::{Method, Expr, LiftedTaskNetwork};
//! let method = Method::new(
//!     Ident::new("example_method"),
//!     TypedList::new(vec![]),
//!     Expr::empty_or(),
//!     Expr::empty_or(),
//!     LiftedTaskNetwork::default(),
//! );
//! println!("Method name: {}", method.name());
//! ```

use crate::aiplan4rust::lang::typed_list::TypedList;
use crate::aiplan4rust::lang::{MethodSymbolId, TaskLabelSymbolId, TypeId, VariableId};
use crate::aiplan4rust::lir::expr::ExprId;
use crate::aiplan4rust::lir::problem::skeleton::named_typed_list::NamedTypedList;
use crate::aiplan4rust::lir::problem::SymbolRegistry;
use crate::aiplan4rust::lir::problem::TaskNetwork;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext};
use core::fmt::Formatter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Method {
    /// The method's header, containing its name and parameters.
    header: NamedTypedList<MethodSymbolId>,

    /// The task expression this method decomposes.
    task: ExprId,

    /// The precondition expression required for the method to be applicable.
    /// This is never `None` and defaults to an empty `Or` expression.
    precondition: ExprId,

    /// The lifted task network describing the subtasks for decomposition.
    task_network: TaskNetwork,

    variable_symbols: SymbolRegistry<VariableId>,
    task_label_symbols: SymbolRegistry<TaskLabelSymbolId>,
}

#[allow(dead_code)]
impl Method {
    /// Constructs a new `Method` with the specified name, parameters, task, precondition, and task network.
    ///
    /// # Parameters
    /// - `name`: The method’s identifier.
    /// - `parameters`: Typed list of parameters for the method.
    /// - `task`: The task expression refined or decomposed by this method.
    /// - `precondition`: Preconditions for method applicability.
    /// - `task_network`: The task network describing subtasks and constraints.
    ///
    /// # Returns
    /// A new `Method` instance.
    pub fn new(
        name: MethodSymbolId,
        parameters: TypedList<VariableId, TypeId>,
        task: ExprId,
        precondition: ExprId,
        task_network: TaskNetwork,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            task,
            precondition,
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

    /// Returns the method's name as an identifier.
    pub fn name(&self) -> MethodSymbolId {
        self.header.symbol()
    }

    /// Sets the method's name.
    pub fn set_name(&mut self, name: MethodSymbolId) {
        self.header.set_symbol(name);
    }

    /// Returns a reference to the action's typed parameters.
    ///
    /// These represent the variables and their types used by the action,
    /// encapsulated in a `TypedList`.
    pub fn parameters(&self) -> &TypedList<VariableId, TypeId> {
        self.header.parameters()
    }

    /// Returns a mutable reference to the action's typed parameters.
    ///
    /// This allows for in-place modification of the parameters (such as typing flattening)
    /// while maintaining the integrity of the `TypedList` structure.
    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableId, TypeId> {
        self.header.parameters_mut()
    }

    /// Sets the method's parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableId, TypeId>) {
        self.header.set_parameters(parameters);
    }

    /// Sets the task expression that this method refines.
    pub fn set_task(&mut self, task: ExprId) {
        self.task = task;
    }

    /// Returns an immutable reference to the task expression.
    pub fn task(&self) -> ExprId {
        self.task
    }

    /// Returns a reference to the method's precondition expression.
    pub fn precondition(&self) -> ExprId {
        self.precondition
    }

    /// Replaces the method's precondition expression.
    pub fn set_precondition(&mut self, pre: ExprId) {
        self.precondition = pre;
    }

    /// Sets the task network representing subtasks and constraints.
    pub fn set_task_network(&mut self, task_network: TaskNetwork) {
        self.task_network = task_network;
    }

    /// Returns an immutable reference to the task network.
    pub fn task_network(&self) -> &TaskNetwork {
        &self.task_network
    }

    /// Returns a mutable reference to the task network.
    pub fn task_network_mut(&mut self) -> &mut TaskNetwork {
        &mut self.task_network
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

impl LiftedSyntaxDisplay for Method {
    /// Rendu HDDL propre (ex: (:method name :parameters (...) :task (...) :precondition (...) :subtasks (...)))
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::syntax::method::render(f, self, ctx)
    }
}

impl LiftedDebugDisplay for Method {
    /// Rendu structurel pour le debug (Method ID, Task Refined, Precondition Tree, TaskNetwork Tree)
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> std::fmt::Result {
        renderers::debug::method::render(f, self, ctx)
    }
}
