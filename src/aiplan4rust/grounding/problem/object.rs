use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::ids::{ObjectID, TypeID};

/// Représente un objet (constant ou objet déclaré dans le problème)
///
/// Chaque objet est associé à :
/// - un **symbol** (ObjectID) correspondant à son index dans la table des objets
/// - un **type** (TypeID) qui indique sa catégorie ou type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Object {
    /// Symbol index dans la table des symboles des objets
    symbol: ObjectID,
    /// Type de l'objet
    ty: TypeID,
}

impl Object {
    /// Crée un nouvel objet avec le symbole et le type spécifiés.
    ///
    /// # Exemple
    ///
    /// ```
    /// let obj = Object::new(ObjectID(0), TypeID(1));
    /// assert_eq!(obj.symbol().0, 0);
    /// assert_eq!(obj.ty().0, 1);
    /// ```
    pub fn new(symbol: ObjectID, ty: TypeID) -> Self {
        Self { symbol, ty }
    }

    /// Retourne le symbol (ObjectID) de l'objet
    pub fn symbol(&self) -> ObjectID {
        self.symbol
    }

    /// Retourne le type (TypeID) de l'objet
    pub fn ty(&self) -> TypeID {
        self.ty
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Affiche le symbole suivi du type
        write!(f, "{} : {}", self.symbol, self.ty)
    }
}
