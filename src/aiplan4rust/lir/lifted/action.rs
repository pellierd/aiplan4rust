use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::NamedTypedList;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Represents an instantaneous action with always-present (possibly empty) precondition and effect.
///
/// By convention, an “empty” expression is represented as an `Or` node with no children.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Action {
    header: NamedTypedList,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    /// Effect expression (never `None`; defaults to empty `Or`).
    effect: Expr,
}

#[allow(dead_code)]
impl Action {
    /// Create a new `Action` with the given name and parameters.
    ///
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: TypedList, precondition: Expr, effect: Expr) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            precondition,
            effect,
        }
    }

    pub fn signature(&self) -> &NamedTypedList {
        &self.header
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
        let signature = NamedTypedList::from_ast(node, ast)?;
        let def_body_node = ast.try_node(node.try_child(2)?)?;

        let mut precondition = Expr::empty_or();
        let mut effect = Expr::empty_or();

        for &child_id in def_body_node.children() {
            let child_node = ast.try_node(child_id)?;
            match child_node.kind() {
                AstKind::PreconditionDef => {
                    let pre_node_id = child_node.try_child(0)?;
                    let pre_node = ast.try_node(pre_node_id)?;
                    precondition = Expr::from_ast(pre_node, ast)?;
                }
                AstKind::EffectDef => {
                    let eff_node_id = child_node.try_child(0)?;
                    let eff_node = ast.try_node(eff_node_id)?;
                    effect = Expr::from_ast(eff_node, ast)?;
                }
                _ => {
                    return Err(ParserInternalError::new(format!(
                        "Unexpected node in Action body: {:?}",
                        child_node.kind()
                    )));
                }
            }
        }

        Ok(Action {
            header: signature,
            precondition,
            effect,
        })
    }

}
#[allow(dead_code)]
impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .parameters()
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "########################################")?;
        writeln!(f, "### ACTION [{}]", self.name())?;
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
            .parameters()
            .iter()
            .map(|p| p.to_string_with_interner(interner))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "########################################")?;
        writeln!(
            f,
            "### ACTION [{}]",
            self.name().to_string_with_interner(interner)
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

impl DisplaySyntax for Action {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
