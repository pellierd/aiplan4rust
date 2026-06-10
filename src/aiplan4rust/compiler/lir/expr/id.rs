//! Unique identifiers for Hash-Consed expressions.

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// On importe uniquement la sentinelle unique à jour
use crate::aiplan4rust::support::lang::ids::RAW_NONE;

// Invocation propre avec Serde personnalisé
crate::impl_id_type!(ExprId, custom_serde);

impl ExprId {
    pub const EMPTY_AND: Self = Self::new(0);
    pub const EMPTY_OR: Self = Self::new(1);

    pub const TRUE: Self = Self::EMPTY_AND;
    pub const FALSE: Self = Self::EMPTY_OR;
}

impl Serialize for ExprId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // On sérialise uniquement l'index propre (sans les bits de flags temporels/négatifs du haut)
        serializer.serialize_str(&self.as_usize().to_string())
    }
}

impl<'de> Deserialize<'de> for ExprId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parsed_value = s.parse::<usize>().map_err(D::Error::custom)?;

        // Si la valeur lue est la sentinelle brute, c'est un NONE
        if parsed_value == RAW_NONE {
            Ok(Self::NONE)
        } else {
            Ok(Self::new(parsed_value))
        }
    }
}

impl fmt::Display for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_some() {
            write!(f, "#{}", self.as_usize())
        } else {
            write!(f, "#NONE")
        }
    }
}
