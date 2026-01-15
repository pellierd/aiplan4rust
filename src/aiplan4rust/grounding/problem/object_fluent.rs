use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::grounding::problem::ids::{FunctionID, ObjectID, TypeID};

/// Représente un object-fluent (fonction non-numérique)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectFluent {
    /// Symbol index dans la table des symboles des object-fluents
    symbol: FunctionID,
    /// Paramètres du fluent, tous des ObjectID (pas d'object-fluent pour éviter récursion)
    parameters: Vec<ObjectID>,
    /// Type de retour du fluent
    ty: TypeID,
}

impl ObjectFluent {
    /// Constructeur pour créer un ObjectFluent
    pub fn new(symbol: FunctionID, parameters: Vec<ObjectID>, ty: TypeID) -> Self {
        Self { symbol, parameters, ty }
    }

    /// Returns the symbol (FunctionID) of the object-fluent.
    pub fn symbol(&self) -> FunctionID {
        self.symbol
    }

    /// Returns a reference to the list of parameters (ObjectID) of the object-fluent.
    pub fn parameters(&self) -> &Vec<ObjectID> {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters of the object-fluent.
    pub fn parameters_mut(&mut self) -> &mut Vec<ObjectID> {
        &mut self.parameters
    }

    /// Replaces the current list of parameters with the provided one.
    pub fn set_parameters(&mut self, params: Vec<ObjectID>) {
        self.parameters = params;
    }

    /// Returns the return type (TypeID) of the object-fluent.
    pub fn ty(&self) -> TypeID {
        self.ty
    }

    /// Sets the return type of the object-fluent.
    pub fn set_ty(&mut self, ty: TypeID) {
        self.ty = ty;
    }

}

impl fmt::Display for ObjectFluent {
    /// Formats the object-fluent in a PDDL-friendly style: `symbol(param1, param2, ...)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.symbol.0)?;
        let mut first = true;
        for param in &self.parameters {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", param.0)?;
            first = false;
        }
        write!(f, ")")
    }
}
