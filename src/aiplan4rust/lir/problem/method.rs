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
use crate::aiplan4rust::interner::ident::Ident;
use crate::aiplan4rust::lang::typed_list::TypedList;
use crate::aiplan4rust::lang::typed_symbol::TypedSymbol;
use crate::aiplan4rust::lir::atomic_skeleton::named_typed_list::NamedTypedList;
use crate::aiplan4rust::lir::expr::expr::Expr;
use crate::aiplan4rust::syntax::ast::node::AstNode;
use crate::aiplan4rust::syntax::display::SyntaxDisplay;
use crate::aiplan4rust::core::arena::node::ArenaNode;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::problem::{normalize, renderers, LiftedTaskNetwork};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::subtree::SyntaxSubtree;

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
        name: Ident,
        parameters: TypedList,
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

    /// Returns the method's name as an identifier.
    pub fn name(&self) -> Ident {
        self.header.name()
    }

    /// Sets the method's name.
    pub fn set_name(&mut self, name: Ident) {
        self.header.set_name(name);
    }

    /// Returns a slice of the method's parameters.
    pub fn parameters(&self) -> &[TypedSymbol] {
        &self.header.parameters()
    }

    /// Sets the method's parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
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

/// Attempts to construct a [`Method`] from a given [`SyntaxSubtree`]
/// referencing an AST node and its syntax tree.
///
/// # Expected AST Structure
/// - The root node represents a method definition.
/// - Child 0: method name (`Ident`)
/// - Child 1: typed parameter list
/// - Child 2: method body node, containing:
///     - Task expression
///     - Optionally, a precondition (`MethodPreconditionDef`)
///     - The lifted task network
///
/// # Returns
/// - `Ok(Method)` on success.
/// - `Err(AiplanError)` if the structure is invalid or parsing fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let method = Method::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for Method {
    type Error = LirError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        // Parse header (name + parameters)
        let header = NamedTypedList::try_from(subtree)?;

        // Parse method body
        let def_body_node = ast.try_node(node.try_child(2)?)?;
        let children = def_body_node.children();

        let mut child_index = 0;

        // Parse the task expression
        let task_node = ast.try_node(children[child_index])?;
        let task = Expr::try_from(&SyntaxSubtree::new(task_node, ast))?;
        child_index += 1;

        // Parse optional precondition
        let precondition = if children.len() > child_index {
            let pre_node_def = ast.try_node(children[child_index])?;
            match pre_node_def.kind() {
                AstKind::MethodPreconditionDef => {
                    let pre_node_id = pre_node_def.try_child(0)?;
                    let pre_node = ast.try_node(pre_node_id)?;
                    child_index += 1;
                    Expr::try_from(&SyntaxSubtree::new(pre_node, ast))?
                }
                _ => Expr::empty_or(),
            }
        } else {
            Expr::empty_or()
        };

        // Parse task network
        let tw_node_def = ast.try_node(children[child_index])?;
        let task_network = LiftedTaskNetwork::try_from(&SyntaxSubtree::new(tw_node_def, ast))?;

        Ok(Method {
            header,
            task,
            precondition,
            task_network,
        })
    }
}

impl fmt::Display for Method {
    /// Formats the `Method` for human-readable output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_method(f, self)
    }
}

impl InternerDisplay for Method {
    /// Formats the `Method` using a string interner for name resolution.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
       renderers::interner::render_method(f, self, interner)
    }
}

impl SyntaxDisplay for Method {
    /// Formats the `Method` syntax with a string interner.
    fn fmt_syntax_with_indent(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        // Write the indentation prefix
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
