use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::{FunctionSymbolId, ObjectId, TypeId};

/// Représente un numeric-fluent (fonction numérique)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NumericFluent {
    /// Symbol index dans la table des symboles des numeric-fluents
    symbol: FunctionSymbolId,
    /// Paramètres de la fonction (toujours ObjectID)
    arguments: Vec<ObjectId>,
    /// Type de retour de la fonction
    ty: TypeId,
}

impl NumericFluent {
    /// Constructeur
    pub fn new(symbol: FunctionSymbolId, parameters: Vec<ObjectId>, ty: TypeId) -> Self {
        Self { symbol, arguments: parameters, ty }
    }

    /// Accès au symbole (FunctionID)
    pub fn symbol(&self) -> FunctionSymbolId {
        self.symbol
    }

    /// Accès aux paramètres
    pub fn arguments(&self) -> &Vec<ObjectId> {
        &self.arguments
    }

    /// Accès mutable aux paramètres
    pub fn arguments_mut(&mut self) -> &mut Vec<ObjectId> {
        &mut self.arguments
    }

    /// Remplace les paramètres
    pub fn set_arguments(&mut self, params: Vec<ObjectId>) {
        self.arguments = params;
    }

    /// Accès au either_type de retour
    pub fn ty(&self) -> TypeId {
        self.ty
    }

    /// Modifie le either_type de retour
    pub fn set_ty(&mut self, ty: TypeId) {
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
