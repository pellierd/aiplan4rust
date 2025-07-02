
use crate::aiplan4rust::ir::expr::Expr;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Method {
    /// The action’s name, e.g. `"move"`.
    name: Ident,

    /// Formal parameters, e.g. `?x - location`.
    parameters: TypedList,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,


}

impl Method {
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: TypedList, precondition: Expr) -> Self {
        Self {
            name,
            parameters,
            precondition,
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

}
