//! Unique identifiers for Hash-Consed expressions.
//!
//! This module defines the [`ExprId`] type, which wraps a `usize` index to uniquely
//! identify expressions stored within a [`Store`].
//!
//! # Key Features
//! - Strong typing: prevents mixing raw indices or other ID types.
//! - Sentinel value: `usize::MAX` represents an invalid or null expression.
//! - String Serialization: Serializes as a string for better compatibility and
//!   readability in JSON (C++ side).

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A unique identifier for an expression within the HcStore.
///
/// `ExprId` wraps a `usize` index. It provides type safety and ensures that
/// expressions are referred to by their canonical ID in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ExprId {
    /// The underlying integer value representing the expression index.
    value: usize,
}

impl Default for ExprId {
    /// Returns an invalid `ExprId` with the sentinel value `usize::MAX`.
    fn default() -> Self {
        ExprId { value: usize::MAX }
    }
}

impl ExprId {
    /// On réserve les 2 bits de poids faible (ou fort selon ton choix final).
    /// Ici, on part sur une réserve de 2 bits, donc l'index max est 2^(bits-2) - 1.
    pub const MAX_INDEX: usize = usize::MAX >> 2;
    /// ID 0 réservé par convention au "And vide" (True)
    pub const EMPTY_AND: Self = Self::new(0);
    /// ID 1 réservé par convention au "Or vide" (False)
    pub const EMPTY_OR: Self = Self::new(1);

    pub const NONE: Self = Self::new(usize::MAX);

    // Alias sémantiques
    pub const TRUE: Self = Self::EMPTY_AND;
    pub const FALSE: Self = Self::EMPTY_OR;

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
    pub fn is_some(self) -> bool {
        self != Self::NONE
    }

    pub const fn new(value: usize) -> Self {
        // On ne peut pas injecter `value` dans le message en const
        debug_assert!(
            value <= Self::MAX_INDEX || value == usize::MAX,
            "ExprId overflow: index exceeds the limit authorized for bit-packing"
        );

        ExprId { value }
    }

    /// Returns the raw `usize` value for indexing into the Store.
    pub fn as_usize(&self) -> usize {
        self.value
    }

    /// Returns `true` if the `ExprId` is valid (not the sentinel value).
    pub fn is_valid(&self) -> bool {
        self.value != usize::MAX
    }
}

// Serialization: ExprId -> "42"
impl Serialize for ExprId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value.to_string())
    }
}

// Deserialization: "42" -> ExprId { value: 42 }
impl<'de> Deserialize<'de> for ExprId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = s.parse::<usize>().map_err(D::Error::custom)?;
        Ok(ExprId { value })
    }
}

impl std::fmt::Display for ExprId {
    /// Friendly display: #42 or #invalid
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "#{}", self.value)
        } else {
            write!(f, "#invalid")
        }
    }
}

impl From<usize> for ExprId {
    fn from(value: usize) -> Self {
        ExprId::new(value)
    }
}

impl From<ExprId> for usize {
    fn from(id: ExprId) -> usize {
        id.value
    }
}
