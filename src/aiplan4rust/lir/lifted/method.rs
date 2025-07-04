use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::lifted::LiftedTaskNetwork;
use crate::aiplan4rust::lir::NamedTypedList;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Method {
    header: NamedTypedList,

    task: Expr,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    task_network: LiftedTaskNetwork
}

#[allow(dead_code)]
impl Method {
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: TypedList, task: Expr, precondition: Expr, task_network: LiftedTaskNetwork) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            task,
            precondition,
            task_network,

        }
    }

    /// Returns the action’s name.
    pub fn name(&self) -> Ident {
        self.header.name()
    }

    /// Sets the action’s name.
    pub fn set_name(&mut self, name: Ident) {
        self.header.set_name(name);
    }

    /// Returns a slice of the action’s parameters.
    pub fn parameters(&self) -> &[TypedSymbol] {
        &self.header.parameters()
    }

    /// Sets the action’s parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.header.set_parameters(parameters);
    }

    /// Sets the `task` expression.
    pub fn set_task(&mut self, task: Expr) {
        self.task = task;
    }

    /// Returns an immutable reference to the `task` expression.
    pub fn task(&self) -> &Expr {
        &self.task
    }

    /// Returns a mutable reference to the `task` expression.
    pub fn task_mut(&mut self) -> &mut Expr {
        &mut self.task
    }

    /// Returns a reference to the precondition expression.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

    /// Sets the `task_network`.
    pub fn set_task_network(&mut self, task_network: LiftedTaskNetwork) {
        self.task_network = task_network;
    }

    /// Returns an immutable reference to the `task_network`.
    pub fn task_network(&self) -> &LiftedTaskNetwork {
        &self.task_network
    }

    /// Returns a mutable reference to the `task_network`.
    pub fn task_network_mut(&mut self) -> &mut LiftedTaskNetwork {
        &mut self.task_network
    }
}

impl FromAst for Method {

    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        // 1. Parse the header (NamedTypedList) from the top-level node
        let header = NamedTypedList::from_ast(node, ast)?;

        // 2. Get the node representing the method body (children container)
        let def_body_node = ast.try_node(node.try_child(2)?)?;
        let children = def_body_node.children();

        // 3. Initialize index to track which child we're processing
        let mut child_index = 0;

        // 4. The first child node is always the task expression
        let task = Expr::from_ast(ast.try_node(children[child_index])?, ast)?;
        child_index += 1;

        // 5. Next, determine if the second child is a precondition
        let pre_node_def = ast.try_node(children[child_index])?;
        let precondition = match pre_node_def.kind() {
            // 5a.️ If it's a PreconditionDef, parse the contained expression
            AstKind::MethodPreconditionDef => {
                let pre_node_id = pre_node_def.try_child(0)?;
                let pre_node = ast.try_node(pre_node_id)?;
                child_index += 1; // Advance because we consumed this node
                Expr::from_ast(pre_node, ast)?
            }
            // 5b. Otherwise, no precondition was specified
            _ => Expr::empty_or(),
        };

        // 6. The next child must be the task network definition
        let tw_node_def = ast.try_node(children[child_index])?;
        let task_network = LiftedTaskNetwork::from_ast(tw_node_def, ast)?;

        // 7/ Build and return the Method object
        Ok(Method {
            header,
            task,
            precondition,
            task_network,
        })
    }

}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .parameters()
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "################# METHOD ##################")?;
        writeln!(f, "NAME [{}]", self.name())?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "TASK [{}]", self.task() )?;
        writeln!(f, "PRECONDITION")?;
        writeln!(f, "{}", self.precondition())?;
        writeln!(f, "{}", self.task_network())?;
        Ok(())
    }
}

impl DisplayWithInterner for Method {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        let params = self
            .parameters()
            .iter()
            .map(|p| p.to_string_with_interner(interner))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "################# METHOD ##################")?;
        writeln!(
            f,
            "NAME [{}]",
            self.name().to_string_with_interner(interner)
        )?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "TASK [{}]", self.task() )?;
        writeln!(f, "PRECONDITIONS")?;
        self.precondition().fmt_with(f, interner)?;
        self.task_network().fmt_with(f, interner)?;
        Ok(())
    }
}

impl DisplaySyntax for Method {
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
