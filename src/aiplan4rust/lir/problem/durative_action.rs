//! Module for lifted PDDL durative actions.
//!
//! This module defines [`DurativeAction`], which extends a lifted action
//! (`LiftedAction`) with a temporal duration and timed conditions.
//! It provides methods to access and modify the action's components,
//! normalize expressions, and render the action in human-readable or PDDL-like syntax.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{TypeID, TypedList};
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lang::{StringID, RemapTypes, Type};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::{normalize, renderers, LiftedAction};
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// Represents a lifted PDDL durative action.
///
/// A `DurativeAction` combines a standard lifted action (`LiftedAction`) with a
/// temporal duration expression. It is part of the internal intermediate
/// representation used in AIPlan4Rust for planning and grounding.
///
/// # Fields
///
/// - `action`: The underlying [`LiftedAction`] containing the action's name,
///   parameters, preconditions (conditions), and effects.
/// - `duration`: An [`Expr`] representing the temporal duration of the action.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::lir::DurativeAction;
/// use aiplan4rust::lang::{Ident, TypedList};
/// use aiplan4rust::lir::expr::Expr;
///
/// let da = DurativeAction::new(
///     Ident::new("load_truck"),
///     TypedList::empty(),
///     Expr::empty_or(),
///     Expr::empty_or(),
///     Expr::empty_or(),
/// );
/// println!("{}", da.duration());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DurativeAction {
    action: LiftedAction,
    duration: Expr,
}

impl DurativeAction {
    /// Creates a new `DurativeAction` with the given name, parameters, duration,
    /// condition, and effect.
    ///
    /// This is the primary public constructor for `DurativeAction`. It builds
    /// the action header from the provided name and parameters, then delegates
    /// to an internal constructor to assemble the full action. This design
    /// avoids code duplication and keeps internal construction logic centralized.
    ///
    /// # Arguments
    ///
    /// * `name` - The identifier/name of the durative action.
    /// * `parameters` - Typed list of parameters for the action.
    /// * `duration` - Expression representing the duration of the action.
    /// * `condition` - Expression representing the timed conditions of the action.
    /// * `effect` - Expression representing the effect of the action.
    ///
    /// # Returns
    ///
    /// A new `DurativeAction` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aiplan4rust::lir::DurativeAction;
    /// # use aiplan4rust::lang::{Ident, TypedList};
    /// # use aiplan4rust::lir::expr::Expr;
    /// let da = DurativeAction::new(
    ///     Ident::new("move"),
    ///     TypedList::empty(),
    ///     Expr::empty_or(),
    ///     Expr::empty_or(),
    ///     Expr::empty_or(),
    /// );
    /// ```
    pub fn new(
        name: StringID,
        parameters: TypedList<TypeID>,
        duration: Expr,
        condition: Expr,
        effect: Expr,
    ) -> Self {
        // Crée le header via NamedTypedList et délègue à from_header
        let header = NamedTypedList::new(name, parameters);
        Self::from_header(header, duration, condition, effect)
    }

    /// Internal constructor using an already-built header.
    ///
    /// This constructor is intended for **internal use only** within the crate.
    /// It allows creating a `DurativeAction` without rebuilding or cloning the
    /// `NamedTypedList` header, which is useful during transformations such as
    /// grounding, normalization, or compilation to other representations.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed action header (name + parameters).
    /// * `duration` - Expression representing the duration of the action.
    /// * `condition` - Expression representing the timed conditions.
    /// * `effect` - Expression representing the effect.
    ///
    /// # Returns
    ///
    /// A new `DurativeAction` instance taking ownership of the provided header.
    pub(crate) fn from_header(
        header: NamedTypedList,
        duration: Expr,
        condition: Expr,
        effect: Expr,
    ) -> Self {
        Self {
            action: LiftedAction::from_header(header, condition, effect),
            duration,
        }
    }

    /// Returns a reference to the action's signature (name + parameters).
    pub fn signature(&self) -> &NamedTypedList {
        &self.action.signature()
    }

    /// Returns the name of the action.
    pub fn name(&self) -> StringID {
        self.action.name()
    }

    /// Sets the name of the action.
    ///
    /// # Parameters
    /// - `name`: The new identifier for the action.
    pub fn set_name(&mut self, name: StringID) {
        self.action.set_name(name);
    }

    /// Returns a slice of the action's parameters.
    pub fn parameters(&self) -> &[TypedSymbol<TypeID>] {
        &self.action.parameters()
    }

    /// Sets the parameters of the action.
    ///
    /// # Parameters
    /// - `parameters`: A typed list of symbols to replace the action's current parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<TypeID>) {
        self.action.set_parameters(parameters);
    }

    /// Returns a reference to the action's duration expression.
    pub fn duration(&self) -> &Expr {
        &self.duration
    }

    /// Returns a mutable reference to the action's duration expression.
    pub fn duration_mut(&mut self) -> &mut Expr {
        &mut self.duration
    }

    /// Sets the action's duration expression.
    ///
    /// # Parameters
    /// - `duration`: The new expression representing the action's duration.
    pub fn set_duration(&mut self, duration: Expr) {
        self.duration = duration;
    }

    /// Returns a reference to the action's timed conditions expression.
    pub fn condition(&self) -> &Expr {
        &self.action.precondition()
    }

    /// Returns a mutable reference to the action's timed conditions expression.
    pub fn condition_mut(&mut self) -> &mut Expr {
        self.action.precondition_mut()
    }

    /// Sets the action's timed conditions expression.
    ///
    /// # Parameters
    /// - `pre`: The new expression representing the action's timed conditions.
    pub fn set_condition(&mut self, pre: Expr) {
        self.action.set_precondition(pre);
    }

    /// Returns a reference to the action's effect expression.
    pub fn effect(&self) -> &Expr {
        &self.action.effect()
    }

    /// Returns a mutable reference to the action's effect expression.
    pub fn effect_mut(&mut self) -> &mut Expr {
        self.action.effect_mut()
    }

    /// Sets the action's effect expression.
    ///
    /// # Parameters
    /// - `eff`: The new expression representing the action's effect.
    pub fn set_effect(&mut self, eff: Expr) {
        self.action.set_effect(eff);
    }

    /// Normalizes all expressions of the durative action.
    ///
    /// This includes the duration, conditions, and effect expressions. Normalization
    /// typically rewrites expressions into a canonical form for consistent processing
    /// (e.g., grounding, simplification, or compilation).
    ///
    /// # Returns
    ///
    /// Returns a [`LirError`] if normalization of any of the expressions fails.
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_durative_action(self)?)
    }
}

/*impl RemapTypes for DurativeAction {
    /// Remaps union types (`Type::Either`) in the action's parameters, conditions, effects, and duration.
    ///
    /// This method updates all `Type::Either` occurrences in the `DurativeAction`:
    /// - The action's parameters
    /// - The precondition (via `condition()`)
    /// - The effect
    /// - The duration expression
    ///
    /// # Parameters
    /// - `map`: A [`HashMap<Type, StringID>`] mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during remapping (e.g., a union type has no corresponding mapping).
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        self.action.remap_types(map)?;
        Ok(())
    }
}*/

/// Implements [`std::fmt::Display`] for `DurativeAction`.
///
/// Provides a human-readable, default representation of the durative action.
/// Suitable for debugging or logging purposes.
impl fmt::Display for DurativeAction {
    /// Writes the action to the formatter.
    ///
    /// # Parameters
    /// - `f`: The formatter to write into.
    ///
    /// # Returns
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderers::default::render_durative_action(f, self)
    }
}

/*/// Implements [`InternerDisplay`] for `DurativeAction`.
///
/// Renders the action using a [`StringInterner`] to resolve interned identifiers,
/// producing readable names for the action's name and parameters.
impl InternerDisplay for DurativeAction {
    /// Writes the action using the provided interner.
    ///
    /// # Parameters
    /// - `f`: The formatter to write into.
    /// - `interner`: Interner used to resolve identifiers.
    ///
    /// # Returns
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderers::interner::render_durative_action(f, self, interner)
    }
}*/

/// Implements [`SyntaxInternerDisplay`] for `DurativeAction`.
///
/// Renders the action in a PDDL-like syntax using a [`StringInterner`] and indentation.
impl SyntaxInternerDisplay for DurativeAction {
    /// Writes the action in PDDL-like syntax with indentation.
    ///
    /// # Parameters
    /// - `f`: The formatter to write into.
    /// - `interner`: Interner used to resolve identifiers.
    /// - `indent`: Number of spaces for indentation.
    ///
    /// # Returns
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        renderers::syntax_old::render_durative_action(f, self, interner, indent)
    }
}
