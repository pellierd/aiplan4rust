use std::fmt;
use std::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

/// Trait pour tous les ID wrappers
pub trait Id: Copy + Eq + Default + std::hash::Hash + Serialize {


/// Constructeur "friendly" identique à from_usize
    fn new(idx: usize) -> Self {

        Self::from_usize(idx)
    }
    fn from_usize(idx: usize) -> Self;
    fn as_usize(self) -> usize;

}

/// Wrappers pour les différents types d’ID
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeID(pub usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateID(pub usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionID(pub usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NumericFluentID(pub usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectID(pub usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectFluentID(pub usize);

// Implémentation du trait Id pour chaque wrapper
macro_rules! impl_id_trait {
    ($id:ty) => {
        impl Id for $id {
            fn from_usize(idx: usize) -> Self { <$id>::from_usize(idx) }
            fn as_usize(self) -> usize { self.0 }
        }

        impl $id {
            /// Constructeur direct
            pub fn new(idx: usize) -> Self {
                <$id>::from_usize(idx)
            }
        }
    };
}

// On peut juste lister tous les IDs
impl_id_trait!(TypeID);
impl_id_trait!(PredicateID);
impl_id_trait!(FunctionID);
impl_id_trait!(NumericFluentID);
impl_id_trait!(ObjectID);
impl_id_trait!(ObjectFluentID);

// --- Index/IndexMut pour chaque wrapper ---
macro_rules! impl_index {
    ($id:ty) => {
        impl<T> Index<$id> for Vec<T> {
            type Output = T;
            fn index(&self, id: $id) -> &Self::Output {
                &self[id.0]
            }
        }
        impl<T> IndexMut<$id> for Vec<T> {
            fn index_mut(&mut self, id: $id) -> &mut Self::Output {
                &mut self[id.0]
            }
        }
    };
}

impl_index!(TypeID);
impl_index!(PredicateID);
impl_index!(FunctionID);
impl_index!(NumericFluentID);
impl_index!(ObjectID);
impl_index!(ObjectFluentID);

macro_rules! impl_display_id {
    ($id:ty, $name:expr) => {
        impl std::fmt::Display for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}#{}", $name, self.0)
            }
        }
    };
}

impl_display_id!(TypeID, "Type");
impl_display_id!(PredicateID, "Predicate");
impl_display_id!(FunctionID, "Function");
impl_display_id!(NumericFluentID, "NumericFluent");
impl_display_id!(ObjectID, "Object");
impl_display_id!(ObjectFluentID, "ObjectFluent");

/// Paramètre groundé dans un fluent ou action
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterID {
    Object(ObjectID),
    ObjectFluent(ObjectFluentID),
}

impl fmt::Display for ParameterID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterID::Object(obj_id) => write!(f, "{}", obj_id),
            ParameterID::ObjectFluent(objf_id) => write!(f, "{}", objf_id),
        }
    }
}
