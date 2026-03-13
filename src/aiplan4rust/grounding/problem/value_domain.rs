use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::ObjectId;
use std::fmt;

/// Domaine de valeurs pour un either_type donné
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValueDomain {
    objects: Vec<ObjectId>, // objets constants
}

impl ValueDomain {
    pub fn new(mut objects: Vec<ObjectId>) -> Self {
        objects.sort_unstable();
        objects.dedup();

        Self { objects }
    }

    pub fn cardinality(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }


    /// Retourne une référence au vecteur d'objets constants
    pub fn values(&self) -> &[ObjectId] {
        &self.objects
    }

    #[inline(always)]
    pub fn get_value_at(&self, index: usize) -> ObjectId {
        self.objects[index]
    }

    /// Iterateur simple sur tous les ObjectId
    pub fn iter(&self) -> impl Iterator<Item =ObjectId> + '_ {
        self.objects.iter().copied()
    }
}

impl fmt::Display for ValueDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.objects.is_empty() {
            write!(f, "<None>")
        } else {
            let all_ids = self
                .objects
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "{}", all_ids)
        }
    }
}
