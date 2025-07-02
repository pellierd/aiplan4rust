
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lang::TypedSymbol;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lir::NamedTypedList;
use crate::aiplan4rust::lir::lifted::LiftedTaskNetwork;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Method {
    header: NamedTypedList,

    /// Precondition expression (never `None`; defaults to empty `Or`).
    precondition: Expr,

    task_network: LiftedTaskNetwork
}

impl Method {
    /// The precondition and effect default to an empty `Or` expression.
    pub fn new(name: Ident, parameters: TypedList, precondition: Expr, task_network: LiftedTaskNetwork) -> Self {
        Self {
            header: NamedTypedList::new(name, parameters),
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

    /// Returns a reference to the precondition expression.
    pub fn precondition(&self) -> &Expr {
        &self.precondition
    }

    /// Replaces the precondition expression.
    pub fn set_precondition(&mut self, pre: Expr) {
        self.precondition = pre;
    }

}
