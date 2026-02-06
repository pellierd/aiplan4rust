//! Module defining the `Action` struct, representing an instantaneous action in a lifted syntax domain.
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

use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lang::{StringID, TypeID, VariableID};
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::expr::Expr;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lir::problem::normalize;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};

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
    pub fn new(
        name: StringID,
        parameters: TypedList<VariableID, TypeID>,
        precondition: Expr,
        effect: Expr,
    ) -> Self {
        let header = NamedTypedList::new(name, parameters);
        Self::from_header(header, precondition, effect)
    }

    /// Creates a new `Action` from an already constructed action header.
    ///
    /// This constructor is intended for **internal use only** within the crate.
    /// It allows creating an `Action` without rebuilding or cloning the
    /// [`NamedTypedList`] header, which is useful during transformations such as
    /// grounding, normalization, or compilation to other representations.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed action header (name and parameters).
    /// * `precondition` - Expression representing the precondition.
    /// * `effect` - Expression representing the effect.
    ///
    /// # Returns
    ///
    /// A new `Action` instance taking ownership of the provided header.
    ///
    /// # Notes
    ///
    /// This function takes ownership of `header` to avoid unnecessary cloning
    /// and should not be exposed as part of the public API.
    pub(crate) fn from_header(
        header: NamedTypedList,
        precondition: Expr,
        effect: Expr,
    ) -> Self {
        Self {
            header,
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
    pub fn name(&self) -> StringID {
        self.header.symbol()
    }

    /// Sets the name (identifier) of the action.
    ///
    /// # Arguments
    ///
    /// * `name` - The new identifier to assign to the action.
    pub fn set_name(&mut self, name: StringID) {
        self.header.set_name(name);
    }

    /// Returns a slice of the action's typed parameters.
    ///
    /// These represent the variables and their types used by the action.
    pub fn parameters(&self) -> &[TypedSymbol<VariableID, TypeID>] {
        &self.header.parameters()
    }

    /// Sets the action's parameters to a new typed list.
    ///
    /// # Arguments
    ///
    /// * `parameters` - The new list of typed parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableID, TypeID>) {
        self.header.set_parameters(parameters);
    }

    /// Returns a reference to the precondition expression of the action.
    ///
    /// The precondition must hold true for the action to be applicable.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Returns a mutable reference to the precondition expression of the action.
    ///
    /// This allows in-place modifications of the precondition,
    /// for example, to normalize or transform the expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::lir::Action;
    /// # use aiplan4rust::lir::expr::Expr;
    /// let mut action = Action::default();
    /// let pre = action.precondition_mut();
    /// // Modify `pre` directly, e.g., normalize(pre);
    /// ```
    pub fn precondition_mut(&mut self) -> &mut Expr {
        &mut self.precondition
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

    /// Returns a mutable reference to the effect expression of the action.
    ///
    /// This allows in-place modifications of the effect,
    /// for example, to normalize or transform the expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::lir::Action;
    /// # use aiplan4rust::lir::expr::Expr;
    /// let mut action = Action::default();
    /// let eff = action.effect_mut();
    /// // Modify `eff` directly, e.g., normalize(eff);
    /// ```
    pub fn effect_mut(&mut self) -> &mut Expr {
        &mut self.effect
    }

    /// Replaces the effect expression.
    ///
    /// # Arguments
    ///
    /// * `eff` - The new effect expression.
    pub fn set_effect(&mut self, eff: Expr) {
        self.effect = eff;
    }

    /// Normalizes the action in-place by normalizing its precondition and effect.
    ///
    /// This ensures that both expressions are in canonical form.
    ///
    /// # Errors
    ///
    /// Returns an `LirError` if normalization fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut action = Action::default();
    /// action.normalize()?;
    /// ```
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_action(self)?)
    }
}

/*impl RemapTypes for Action {
    /// Remaps union types (`Type::Either`) in the action's parameters, precondition, and effect.
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
        self.effect.remap_types(map)?;
        Ok(())
    }
}*/

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_action(f, self)
    }
}

impl LiftedSyntaxDisplay for Action {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        renderers::syntax::action::render(f, self, ctx)
    }
}
