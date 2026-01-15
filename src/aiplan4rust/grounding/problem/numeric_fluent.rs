use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::grounding::problem::ids::{FunctionID, ObjectID, TypeID};

/// Représente un numeric-fluent (fonction numérique)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NumericFluent {
    /// Symbol index dans la table des symboles des numeric-fluents
    symbol: FunctionID,
    /// Paramètres de la fonction (toujours ObjectID)
    parameters: Vec<ObjectID>,
    /// Type de retour de la fonction
    ty: TypeID,
}

impl NumericFluent {
    /// Constructeur
    pub fn new(symbol: FunctionID, parameters: Vec<ObjectID>, ty: TypeID) -> Self {
        Self { symbol, parameters, ty }
    }

    /// Accès au symbole (FunctionID)
    pub fn symbol(&self) -> FunctionID {
        self.symbol
    }

    /// Accès aux paramètres
    pub fn parameters(&self) -> &Vec<ObjectID> {
        &self.parameters
    }

    /// Accès mutable aux paramètres
    pub fn parameters_mut(&mut self) -> &mut Vec<ObjectID> {
        &mut self.parameters
    }

    /// Remplace les paramètres
    pub fn set_parameters(&mut self, params: Vec<ObjectID>) {
        self.parameters = params;
    }

    /// Accès au type de retour
    pub fn ty(&self) -> TypeID {
        self.ty
    }

    /// Modifie le type de retour
    pub fn set_ty(&mut self, ty: TypeID) {
        self.ty = ty;
    }

}

impl fmt::Display for NumericFluent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Affichage par défaut avec IDs bruts
        write!(f, "{}(", self.symbol.0)?;
        let mut first = true;
        for param in &self.parameters {
            if !first { write!(f, ", ")?; }
            write!(f, "{}", param.0)?;
            first = false;
        }
        write!(f, ") : {}", self.ty.0)
    }
}
