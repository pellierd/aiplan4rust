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
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TypeID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PredicateID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ParameterID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct VariableID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct FunctionID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NumericFluentID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ObjectID(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ObjectFluentID(usize);

macro_rules! impl_id_trait {
    ($id:ty) => {
        impl Id for $id {
            fn new(idx: usize) -> Self { Self(idx) }
            fn from_usize(idx: usize) -> Self { <$id>::new(idx) }
            fn as_usize(self) -> usize { self.0 }
        }

        impl Default for $id {
            fn default() -> Self {
                Self(usize::MAX)
            }
        }

        impl $id {
            /// Constructeur direct
            pub fn new(idx: usize) -> Self {
                Self(idx)
            }
            pub fn is_valid(self) -> bool {
                self.0 != usize::MAX
            }
        }
    };
}

// On peut juste lister tous les IDs
impl_id_trait!(TypeID);
impl_id_trait!(ParameterID);
impl_id_trait!(VariableID);
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
impl_index!(ParameterID);
impl_index!(VariableID);
impl_index!(FunctionID);
impl_index!(NumericFluentID);
impl_index!(ObjectID);
impl_index!(ObjectFluentID);

macro_rules! impl_display_id {
    ($id:ty, $prefix:expr) => {
        impl std::fmt::Display for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}#{}{}", $prefix, self.0, "")
            }
        }
    };
}

impl_display_id!(TypeID, "T");
impl_display_id!(PredicateID, "P");
impl_display_id!(ParameterID, "param");
impl_display_id!(VariableID, "var");
impl_display_id!(FunctionID, "F");
impl_display_id!(NumericFluentID, "NF");
impl_display_id!(ObjectID, "O");
impl_display_id!(ObjectFluentID, "OF");

/// Paramètre groundé dans un fluent ou action
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArgumentID {
    Object(ObjectID),
    ObjectFluent(ObjectFluentID),
}

impl fmt::Display for ArgumentID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentID::Object(obj_id) => write!(f, "{}", obj_id),
            ArgumentID::ObjectFluent(obj_fluent_id) => write!(f, "{}", obj_fluent_id),
        }
    }
}
