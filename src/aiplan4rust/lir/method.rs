//! This module defines the `Method` struct, representing a lifted method in a hierarchical task network (HTN) planning domain.
//!
//! A `Method` describes how a complex task can be decomposed into subtasks under certain preconditions,
//! encapsulating the method's name, parameters, the task it refines, its preconditions, and the resulting task network.
//!
//! This is a core abstraction for expressing domain methods in HTN planning.
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

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LiftedTaskNetwork;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
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

impl FromAst for Method {
    /// Parses a `Method` from its AST representation.
    ///
    /// # Parameters
    /// - `node`: The AST node representing the method.
    /// - `ast`: The arena of AST nodes.
    ///
    /// # Returns
    /// Returns a `Method` instance or a `ParserInternalError` if parsing fails.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        // Parse header (name + parameters)
        let header = NamedTypedList::from_ast(node, ast)?;

        // Parse method body children container
        let def_body_node = ast.try_node(node.try_child(2)?)?;
        let children = def_body_node.children();

        let mut child_index = 0;

        // Parse the task expression (first child)
        let task = Expr::from_ast(ast.try_node(children[child_index])?, ast)?;
        child_index += 1;

        // Parse optional precondition (second child, if present)
        let pre_node_def = ast.try_node(children[child_index])?;
        let precondition = match pre_node_def.kind() {
            AstKind::MethodPreconditionDef => {
                let pre_node_id = pre_node_def.try_child(0)?;
                let pre_node = ast.try_node(pre_node_id)?;
                child_index += 1;
                Expr::from_ast(pre_node, ast)?
            }
            _ => Expr::empty_or(),
        };

        // Parse task network (next child)
        let tw_node_def = ast.try_node(children[child_index])?;
        let task_network = LiftedTaskNetwork::from_ast(tw_node_def, ast)?;

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
        let params = self.parameters()
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "################# METHOD ##################")?;
        writeln!(f, "NAME [{}]", self.name())?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "TASK [{}]", self.task())?;
        writeln!(f, "PRECONDITION")?;
        writeln!(f, "{}", self.precondition())?;
        writeln!(f, "{}", self.task_network())?;
        Ok(())
    }
}

impl DisplayWithInterner for Method {
    /// Formats the `Method` using a string interner for name resolution.
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        let params = self.parameters()
            .iter()
            .map(|p| p.to_string_with_interner(interner))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "################# METHOD ##################")?;
        writeln!(f, "NAME [{}]", self.name().to_string_with_interner(interner))?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "TASK [{}]", self.task())?;
        writeln!(f, "PRECONDITIONS")?;
        self.precondition().fmt_with(f, interner)?;
        self.task_network().fmt_with(f, interner)?;
        Ok(())
    }
}

impl PlanningSyntaxDisplay for Method {
    /// Formats the `Method` syntax with a string interner.
    fn fmt_planning(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
