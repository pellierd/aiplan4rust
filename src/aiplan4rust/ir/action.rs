use crate::aiplan4rust::ir::expr::{Expr, ExprNode, ExprKind, ExprContent};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::semantic::symbol::TypedSymbol;
use crate::aiplan4rust::syntax::elements::Ident;

/// Represents an instantaneous action with always-present (possibly empty) precondition and effect.
///
/// By convention, an “empty” expression is represented as an `Or` node with no children.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Action {
    /// The action’s name, e.g. `"move"`.
    name: Ident,

    /// Formal parameters, e.g. `?x - location`.
    parameters: Vec<TypedSymbol>,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    /// Effect expression (never `None`; defaults to empty `Or`).
    effect: Expr,
}

impl Action {
    /// Create a new `Action` with the given name and parameters.
    ///
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: Vec<TypedSymbol>, precondition: Expr, effect: Expr) -> Self {
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
    pub fn set_parameters(&mut self, params: Vec<TypedSymbol>) {
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


impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .parameters
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "{}({})", self.name, params)?;
        write!(f, " pre: {:?}", self.precondition)?;
        write!(f, " eff: {:?}", self.effect)
    }
}
