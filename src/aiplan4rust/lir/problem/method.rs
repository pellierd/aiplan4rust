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

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::typed_list::TypedList;
use crate::aiplan4rust::lang::typed_symbol::TypedSymbol;
use crate::aiplan4rust::lang::{RemapTypes, StringID, Type, TypeID};
use crate::aiplan4rust::lir::atomic_skeleton::named_typed_list::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::expr::expr::Expr;
use crate::aiplan4rust::lir::problem::{normalize, renderers, LiftedTaskNetwork};
use crate::aiplan4rust::syntax::display::SyntaxInternerDisplay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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
        parameters: TypedList<TypeID>,
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
    /// grounding, normalization, or compilation to other representations.
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

    /// Returns a slice of the method's parameters.
    pub fn parameters(&self) -> &[TypedSymbol<TypeID>] {
        &self.header.parameters()
    }

    /// Sets the method's parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<TypeID>) {
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

    /// Normalizes the method in-place by normalizing its precondition
    /// and task network.
    ///
    /// This ensures that both the `precondition` and the expressions
    /// in the `task_network` are in canonical form.
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if normalization fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut method = Method::default();
    /// method.normalize()?;
    /// ```
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_method(self)?)
    }

}

/*impl RemapTypes for Method {
    /// Remaps union types (`Type::Either`) in the method's header and precondition.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during remapping (e.g., a union type has no corresponding mapping).
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        self.header.remap_types(map)?;
        self.precondition.remap_types(map)?;
        Ok(())
    }
}*/

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

impl InternerDisplay for Method {
    /// Formats the `Method` using a [`StringInterner`] for name resolution.
    ///
    /// This implementation resolves interned identifiers for the method's
    /// name and parameters before rendering, producing a string that
    /// reflects the human-readable names of all symbols.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    /// * `interner` - The [`StringInterner`] used to resolve interned identifiers.
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
    /// # let interner: StringInterner = todo!();
    /// let mut s = String::new();
    /// method.fmt_with_interner(&mut s, &interner).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderers::interner::render_method(f, self, interner)
    }
}

impl SyntaxInternerDisplay for Method {
    /// Formats the `Method` syntax with a [`StringInterner`] and optional indentation.
    ///
    /// This implementation produces a PDDL-like syntax representation of the method,
    /// including its parameters, task, precondition, and task network, indented
    /// according to the `indent` parameter for readability.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    /// * `interner` - The [`StringInterner`] used to resolve interned identifiers.
    /// * `indent` - The indentation level to apply to the rendered syntax.
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
    /// # let interner: StringInterner = todo!();
    /// let mut s = String::new();
    /// method.fmt_syntax_with_interner_and_indent(&mut s, &interner, 2).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize
    ) -> fmt::Result {
        renderers::syntax::render_method(f, self, interner, indent)
    }
}
