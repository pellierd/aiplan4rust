//! Module defining the `Action` struct, representing an instantaneous action in a lifted planning domain.
//!
//! An `Action` includes a name, parameters, a precondition, and an effect expression.
//! Both precondition and effect are always present, defaulting to an empty expression (an `Or` with no children) if unspecified.
//!
//! This module provides:
//! - Construction of actions from parsed AST nodes.
//! - Accessors and mutators for the action's signature, name, parameters, precondition, and effect.
//! - Display implementations for debugging and formatted output, including interner-aware printing.
//!
//! # Structure
//!
//! - `Action` encapsulates the concept of an instantaneous action with:
//!   - A header (`NamedTypedList`) holding the action name and typed parameters.
//!   - A precondition expression that must hold before execution.
//!   - An effect expression describing the outcome of the action.
//!
//! # Conversion from AST
//!
//! The module supports creating an `Action` from a syntax subtree of an AST, extracting the signature, precondition, and effect nodes,
//! with sensible defaults if the precondition or effect are omitted.
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
//!
//! # Error handling
//!
//! Parsing from AST may fail with `LirError` if the structure is invalid or missing expected parts.

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::syntax::SyntaxDisplay;

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
    ///
    /// * `name` - The identifier/name of the action.
    /// * `parameters` - Typed list of parameters for the action.
    /// * `precondition` - Expression representing the precondition.
    /// * `effect` - Expression representing the effect.
    ///
    /// # Returns
    ///
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
    ///
    /// This includes both the action's identifier and its typed parameters.
    pub fn signature(&self) -> &NamedTypedList {
        &self.header
    }

    /// Returns the name (identifier) of the action.
    pub fn name(&self) -> Ident {
        self.header.name()
    }

    /// Sets the name (identifier) of the action.
    ///
    /// # Arguments
    ///
    /// * `name` - The new identifier to assign to the action.
    pub fn set_name(&mut self, name: Ident) {
        self.header.set_name(name);
    }

    /// Returns a slice of the action's typed parameters.
    ///
    /// These represent the variables and their types used by the action.
    pub fn parameters(&self) -> &[TypedSymbol] {
        &self.header.parameters()
    }

    /// Sets the action's parameters to a new typed list.
    ///
    /// # Arguments
    ///
    /// * `parameters` - The new list of typed parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.header.set_parameters(parameters);
    }

    /// Returns a reference to the precondition expression of the action.
    ///
    /// The precondition must hold true for the action to be applicable.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the precondition expression.
    ///
    /// # Arguments
    ///
    /// * `pre` - The new precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

    /// Returns a reference to the effect expression of the action.
    ///
    /// The effect describes how the world changes after executing the action.
    pub fn effect(&self) -> &Expr {
        &self.effect
    }

    /// Replaces the effect expression.
    ///
    /// # Arguments
    ///
    /// * `eff` - The new effect expression.
    pub fn set_effect(&mut self, eff: Expr) {
        self.effect = eff;
    }
}

/// Attempts to construct an [`Action`] from a given [`SyntaxSubtree`] referencing an AST node and its syntax tree.
///
/// # Expected AST Structure
///
/// - Child 0: Action name identifier (`Ident`).
/// - Child 1: Typed parameter list.
/// - Child 2: Body node, which may contain:
///   - A precondition definition node (`PreconditionDef`).
///   - An effect definition node (`EffectDef`).
///
/// If precondition or effect nodes are missing, they default to empty expressions.
///
/// # Errors
///
/// Returns an [`LirError`] if the syntax structure is invalid or if parsing fails.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let action = Action::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for Action {
    type Error = LirError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let node = subtree.node();
        let ast = subtree.tree();

        // Parse the action signature (name + parameters)
        let header = NamedTypedList::try_from(&SyntaxSubtree::new(node, ast))?;

        // Get the body node of the action
        let def_body_node = ast.try_node(node.try_child(2)?)?;

        // Initialize precondition and effect with empty expressions by default
        let mut precondition = Expr::empty_or();
        let mut effect = Expr::empty_or();

        // Iterate over the children of the body node to find precondition and effect
        for &child_id in def_body_node.children() {
            let child_node = ast.try_node(child_id)?;
            match child_node.kind() {
                AstKind::PreconditionDef => {
                    let pre_node_id = child_node.try_child(0)?;
                    let pre_node = ast.try_node(pre_node_id)?;
                    precondition = Expr::try_from(&SyntaxSubtree::new(pre_node, ast))?;
                }
                AstKind::EffectDef => {
                    let eff_node_id = child_node.try_child(0)?;
                    let eff_node = ast.try_node(eff_node_id)?;
                    effect = Expr::try_from(&SyntaxSubtree::new(eff_node, ast))?;
                }
                _ => {
                    return Err(LirError::action_ast_kind_error(child_node.kind()));
                }
            }
        }

        Ok(Action {
            header,
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
    /// Useful for pretty-printing names and parameters with interning.
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
    /// Formats the `Action` syntax for display, delegating to `fmt_with_interner`.
    ///
    /// The output is indented according to the `indent` parameter.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.fmt_with_interner(f, interner)
    }
}
