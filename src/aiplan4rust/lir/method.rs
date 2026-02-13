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
use crate::aiplan4rust::lang::{StringID, TypeID, VariableID};
use crate::aiplan4rust::lir::atomic_skeleton::named_typed_list::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::expr::expr::Expr;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::{passes, renderers, LiftedTaskNetwork};
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Method {
    /// The method's header, containing its name and parameters.
    header: NamedTypedList,

    /// The task expression this method decomposes.
    task: Expr,

    /// The precondition expression required for the method to be applicable.
    /// This is never `None` and defaults to an empty `Or` expression.
    precondition: Expr,

    /// The lifted task network describing the subtasks for decomposition.
    task_network: LiftedTaskNetwork,
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
        name: StringID,
        parameters: TypedList<VariableID, TypeID>,
        task: Expr,
        precondition: Expr,
        task_network: LiftedTaskNetwork,
    ) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            task,
            precondition,
            task_network,
        }
    }

    /// Creates a new `Method` from an already constructed method header.
    ///
    /// This constructor is intended for **internal use only** within the crate.
    /// It allows creating a `Method` without rebuilding or cloning the
    /// [`NamedTypedList`] header, which is useful during transformations such as
    /// grounding, expr, or compilation to other representations.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed method header (name and parameters).
    /// * `task` - The task expression refined or decomposed by this method.
    /// * `precondition` - Expression representing the precondition.
    /// * `task_network` - The lifted task network describing the subtasks.
    ///
    /// # Returns
    ///
    /// A new `Method` instance taking ownership of the provided header.
    ///
    /// # Notes
    ///
    /// This function takes ownership of `header` to avoid unnecessary cloning
    /// and should not be exposed as part of the public API.
    pub(crate) fn from_header(
        header: NamedTypedList,
        task: Expr,
        precondition: Expr,
        task_network: LiftedTaskNetwork,
    ) -> Self {
        Self {
            header,
            task,
            precondition,
            task_network,
        }
    }

    /// Returns the method's name as an identifier.
    pub fn name(&self) -> StringID {
        self.header.symbol()
    }

    /// Sets the method's name.
    pub fn set_name(&mut self, name: StringID) {
        self.header.set_name(name);
    }

    /// Returns a reference to the action's typed parameters.
    ///
    /// These represent the variables and their types used by the action,
    /// encapsulated in a `TypedList`.
    pub fn parameters(&self) -> &TypedList<VariableID, TypeID> {
        self.header.parameters()
    }

    /// Returns a mutable reference to the action's typed parameters.
    ///
    /// This allows for in-place modification of the parameters (such as type flattening)
    /// while maintaining the integrity of the `TypedList` structure.
    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableID, TypeID> {
        self.header.parameters_mut()
    }

    /// Sets the method's parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableID, TypeID>) {
        self.header.set_parameters(parameters);
    }

    /// Sets the task expression that this method refines.
    pub fn set_task(&mut self, task: Expr) {
        self.task = task;
    }

    /// Returns an immutable reference to the task expression.
    pub fn task(&self) -> &Expr {
        &self.task
    }

    /// Returns a mutable reference to the task expression.
    pub fn task_mut(&mut self) -> &mut Expr {
        &mut self.task
    }

    /// Returns a reference to the method's precondition expression.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the method's precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

    /// Returns a mutable reference to the method's precondition expression.
    ///
    /// This allows in-place modifications of the precondition,
    /// for example to normalize or transform the expression.
    pub fn precondition_mut(&mut self) -> &mut Expr {
        &mut self.precondition
    }

    /// Sets the task network representing subtasks and constraints.
    pub fn set_task_network(&mut self, task_network: LiftedTaskNetwork) {
        self.task_network = task_network;
    }

    /// Returns an immutable reference to the task network.
    pub fn task_network(&self) -> &LiftedTaskNetwork {
        &self.task_network
    }

    /// Returns a mutable reference to the task network.
    pub fn task_network_mut(&mut self) -> &mut LiftedTaskNetwork {
        &mut self.task_network
    }

}


impl fmt::Display for Method {
    /// Formats the `Method` for human-readable output.
    ///
    /// This implementation uses the default renderer to produce a readable
    /// representation of the method, including its name, parameters,
    /// task, precondition, and task network.
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
    /// # let method: Method = todo!();
    /// let mut s = String::new();
    /// write!(&mut s, "{}", method).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_method(f, self)
    }
}

impl LiftedSyntaxDisplay for Method {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        renderers::syntax::method::render(f, self, ctx)
    }
}
