use std::fmt;
use crate::aiplan4rust::lang::ids::{
    FunctorID, ObjectID, ParameterID, PredicateID, TypeID, VariableID,
};
use serde::{Deserialize, Serialize};

/// Représente la liaison d'un symbole vers une entité du domaine ou du problème.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resolution {
    /// Pas encore résolu (état initial lors du passage AST -> LIR)
    #[default]
    None,
    /// Un paramètre de l'action, de la tâche ou de la méthode parente
    Parameter(ParameterID, TypeID),
    /// Une variable introduite par un scope (forall, exists, ou variable de méthode HTN)
    Variable(VariableID, TypeID),
    /// Un objet constant global
    Constant(ObjectID, Vec<TypeID>),
    /// Un prédicat déclaré dans le domaine
    Predicate(PredicateID),
    /// Une fonction (numérique ou d'objet) avec son type de retour
    Function(FunctorID, Vec<TypeID>),
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::None => write!(f, "Unresolved"),
            Resolution::Parameter(id, ty) => write!(f, "{}<{}>", id, ty),
            Resolution::Variable(id, ty) => write!(f, "{}<{}>", id, ty),
            Resolution::Constant(id, ty) => {
                let types_str = ty
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}<{}>", id, types_str)
            },
            Resolution::Predicate(id) => write!(f, "{}", id),
            Resolution::Function(id, ty) => {
                let types_str = ty
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}<{}>", id, types_str)
            },
        }
    }
}
