use crate::aiplan4rust::lang::ObjectId;
use ordered_float::OrderedFloat;

/// Représente une valeur constante extraite par une analyse statique (ex: Inertie).
/// Ce typing sert de langage commun entre le LIR et les modules externes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StaticValue {
    /// Valeur booléenne (généralement pour un prédicat statique)
    Boolean(bool),
    /// Valeur numérique (pour une fonction inerte)
    Number(OrderedFloat<f64>),
    /// Référence à un objet constant (pour une fonction inerte retournant un objet)
    Object(ObjectId),
}
