use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::{FunctorID, ObjectID, TypeID};

/// Représente un numeric-fluent (fonction numérique)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NumericFluent {
    /// Symbol index dans la table des symboles des numeric-fluents
    symbol: FunctorID,
    /// Paramètres de la fonction (toujours ObjectID)
    arguments: Vec<ObjectID>,
    /// Type de retour de la fonction
    ty: TypeID,
}

impl NumericFluent {
    /// Constructeur
    pub fn new(symbol: FunctorID, parameters: Vec<ObjectID>, ty: TypeID) -> Self {
        Self { symbol, arguments: parameters, ty }
    }

    /// Accès au symbole (FunctionID)
    pub fn symbol(&self) -> FunctorID {
        self.symbol
    }

    /// Accès aux paramètres
    pub fn arguments(&self) -> &Vec<ObjectID> {
        &self.arguments
    }

    /// Accès mutable aux paramètres
    pub fn arguments_mut(&mut self) -> &mut Vec<ObjectID> {
        &mut self.arguments
    }

    /// Remplace les paramètres
    pub fn set_arguments(&mut self, params: Vec<ObjectID>) {
        self.arguments = params;
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
        write!(f, "{}(", self.symbol)?;
        let mut first = true;
        for arg in &self.arguments {
            if !first { write!(f, ", ")?; }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ") : {}", self.ty)
    }
}
