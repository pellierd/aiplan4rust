use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::ir::expr::Expr;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents an instantaneous action with always-present (possibly empty) precondition and effect.
///
/// By convention, an “empty” expression is represented as an `Or` node with no children.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Action {
    /// The action’s name, e.g. `"move"`.
    name: Ident,

    /// Formal parameters, e.g. `?x - location`.
    parameters: TypedList,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    /// Effect expression (never `None`; defaults to empty `Or`).
    effect: Expr,
}

impl Action {
    /// Create a new `Action` with the given name and parameters.
    ///
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: TypedList, precondition: Expr, effect: Expr) -> Self {
        Self {
            name,
            parameters,
            precondition,
            effect,
        }
    }

    /// Returns the action’s name.
    pub fn name(&self) -> Ident {
        self.name
    }

    /// Sets the action’s name.
    pub fn set_name(&mut self, name: Ident) {
        self.name = name;
    }

    /// Returns a slice of the action’s parameters.
    pub fn parameters(&self) -> &[TypedSymbol] {
        &self.parameters
    }

    /// Sets the action’s parameters.
    pub fn set_parameters(&mut self, params: TypedList) {
        self.parameters = params;
    }

    /// Returns a reference to the precondition expression.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

    /// Returns a reference to the effect expression.
    pub fn effect(&self) -> &Expr {
        &self.effect
    }

    /// Replaces the effect expression.
    pub fn set_effect(&mut self, eff: Expr) {
        self.effect = eff;
    }
}

impl FromAst for Action {
    /// Constructs an `Action` from an AST node.
    ///
    /// # Expected AST structure
    ///
    /// The input `node` should represent an `Action` with exactly three children:
    /// 1. The first child is the name node (identifier).
    /// 2. The second child is the parameters node, which contains a typed list.
    /// 3. The third child is the definition body node, which can have up to two children:
    ///     - The first child (optional) represents the precondition.
    ///     - The second child (optional) represents the effect.
    ///
    /// If the precondition or effect nodes are missing, an empty expression (`Expr::empty_or()`) is used.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        // Retrieve the action name node and extract its identifier
        let name_node = ast.try_node(node.try_child(0)?)?;
        let name = name_node.try_ident()?;

        // Retrieve the parameters node and parse it into a TypedList
        let params_node = ast.try_node(node.try_child(1)?)?;
        let parameters = TypedList::from_ast(&params_node, ast)?;

        // Retrieve the definition body node
        let def_body_node = ast.try_node(node.try_child(2)?)?;

        // Retrieve the precondition if it exists; otherwise use an empty expression
        let precondition = if let Some(pre_def_id) = def_body_node.try_child(0).ok() {
            let pre_def = ast.try_node(pre_def_id)?;
            let pre_id = pre_def.try_child(0)?;
            let pre = ast.try_node(pre_id)?;
            Expr::from_ast(pre, ast)?
        } else {
            Expr::empty_or()
        };

        // Retrieve the effect if it exists; otherwise use an empty expression
        let effect = if let Some(effect_def_id) = def_body_node.try_child(1).ok() {
            let eff_def = ast.try_node(effect_def_id)?;
            let eff_id = eff_def.try_child(0)?;
            let eff = ast.try_node(eff_id)?;
            Expr::from_ast(eff, ast)?
        } else {
            Expr::empty_or()
        };

        Ok(Action::new(name, parameters, precondition, effect))
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .parameters
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "########################################")?;
        writeln!(f, "### ACTION [{}]", self.name)?;
        writeln!(f, "### PARAMETERS [{}]", params)?;
        writeln!(f, "### PRECONDITION")?;
        writeln!(f, "{}", self.precondition)?;
        writeln!(f, "### EFFECT")?;
        writeln!(f, "{}", self.effect)?;
        writeln!(f, "########################################")
    }
}

impl DisplayWithInterner for Action {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        let params = self
            .parameters
            .iter()
            .map(|p| p.to_string_with_interner(interner))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "########################################")?;
        writeln!(
            f,
            "### ACTION [{}]",
            self.name.to_string_with_interner(interner)
        )?;
        writeln!(f, "### PARAMETERS [{}]", params)?;
        writeln!(f, "########################################")?;
        writeln!(f, "### PRECONDITION")?;
        self.precondition.fmt_with(f, interner)?;
        writeln!(f, "########################################")?;
        writeln!(f, "### EFFECT")?;
        self.effect.fmt_with(f, interner)?;
        writeln!(f, "########################################")
    }
}
