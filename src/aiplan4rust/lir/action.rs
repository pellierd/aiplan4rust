//! Module defining the `Action` struct, representing an instantaneous action in the planning domain.
//!
//! An `Action` includes a name, parameters, a precondition, and an effect expression.
//! Both precondition and effect are always present, defaulting to an empty expression if unspecified.
//!
//! This module provides:
//! - Construction of actions from parsed AST nodes.
//! - Accessors and mutators for the action's signature, precondition, and effect.
//! - Display implementations for debugging and formatted output.
//!
//! # Usage example
//!
//! ```rust
//! # use aiplan4rust::lir::Action;
//! # use aiplan4rust::lang::{Ident, TypedList};
//! # use aiplan4rust::lir::expr::Expr;
//! let action = Action::new(
//!     Ident::new("move"),
//!     TypedList::empty(),
//!     Expr::empty_or(),
//!     Expr::empty_or(),
//! );
//! println!("Action name: {}", action.name());
//! ```

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::arena::{Arena, ArenaNode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;

/// Represents an instantaneous action with a name, parameters, precondition, and effect.
///
/// The precondition and effect are always present and default to empty expressions (an `Or` syntax with no children).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Action {
    header: NamedTypedList,

    /// The precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    /// The effect expression (never `None`; defaults to empty `Or`).
    effect: Expr,
}

#[allow(dead_code)]
impl Action {
    /// Creates a new `Action` with the given name, parameters, precondition, and effect.
    ///
    /// # Arguments
    /// * `name` - The identifier/name of the action.
    /// * `parameters` - Typed list of parameters for the action.
    /// * `precondition` - Expression representing the precondition.
    /// * `effect` - Expression representing the effect.
    ///
    /// # Returns
    /// A new `Action` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::lir::Action;
    /// # use aiplan4rust::lang::{Ident, TypedList};
    /// # use aiplan4rust::lir::expr::Expr;
    /// let a = Action::new(
    ///     Ident::new("test_action"),
    ///     TypedList::empty(),
    ///     Expr::empty_or(),
    ///     Expr::empty_or(),
    /// );
    /// ```
    pub fn new(name: Ident, parameters: TypedList, precondition: Expr, effect: Expr) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
            precondition,
            effect,
        }
    }

    /// Returns a reference to the full signature (name + parameters).
    pub fn signature(&self) -> &NamedTypedList {
        &self.header
    }

    /// Returns the name of the action.
    pub fn name(&self) -> Ident {
        self.header.name()
    }

    /// Sets the name of the action.
    ///
    /// # Arguments
    /// * `name` - The new identifier to assign.
    pub fn set_name(&mut self, name: Ident) {
        self.header.set_name(name);
    }

    /// Returns a slice of the action's parameters.
    pub fn parameters(&self) -> &[TypedSymbol] {
        &self.header.parameters()
    }

    /// Sets the action's parameters.
    ///
    /// # Arguments
    /// * `parameters` - The new list of typed parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.header.set_parameters(parameters);
    }

    /// Returns a reference to the precondition expression.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the precondition expression.
    ///
    /// # Arguments
    /// * `pre` - The new precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

    /// Returns a reference to the effect expression.
    pub fn effect(&self) -> &Expr {
        &self.effect
    }

    /// Replaces the effect expression.
    ///
    /// # Arguments
    /// * `eff` - The new effect expression.
    pub fn set_effect(&mut self, eff: Expr) {
        self.effect = eff;
    }
}

impl FromAst for Action {
    /// Constructs an `Action` from an AST syntax.
    ///
    /// # Expected AST structure
    /// The syntax should have:
    /// - The first child: the action name identifier.
    /// - The second child: parameters as a typed list.
    /// - The third child: the body containing optional precondition and effect nodes.
    ///
    /// If the precondition or effect are missing, they default to empty expressions.
    ///
    /// # Errors
    /// Returns `ParserInternalError` if the AST structure is unexpected or parsing fails.
    fn from_ast(
        node: &AstNode,
        ast: &Arena<AstNode>,
    ) -> Result<Self, AiplanError> {
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
                    return Err(AiplanError::InternalError(format!(
                        "Unexpected syntax in Action body: {:?}",
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

impl fmt::Display for Action {
    /// Formats the `Action` for display purposes.
    ///
    /// Prints the name, parameters, precondition, and effect in a human-readable way.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .parameters()
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(f, "################# ACTION ##################")?;
        writeln!(f, "NAME [{}]", self.name())?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "PRECONDITION")?;
        writeln!(f, "{}", self.precondition)?;
        writeln!(f, "EFFECT")?;
        writeln!(f, "{}", self.effect)
    }
}

impl InternerDisplay for Action {
    /// Formats the `Action` using a string interner for symbol resolution.
    ///
    /// This is useful for pretty-printing names and parameters with interning.
    fn fmt_with_interner(
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

        writeln!(f, "################# ACTION ##################")?;
        writeln!(
            f,
            "NAME [{}]",
            self.name().to_string_with_interner(interner)
        )?;
        writeln!(f, "PARAMETERS [{}]", params)?;
        writeln!(f, "PRECONDITION")?;
        self.precondition.fmt_with_interner(f, interner)?;
        writeln!(f, " EFFECT")?;
        self.effect.fmt_with_interner(f, interner)
    }
}

impl SyntaxDisplay for Action {
    /// Formats the `Action` syntax for display, delegating to `fmt_with`.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
