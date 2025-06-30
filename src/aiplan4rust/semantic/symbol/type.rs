use std::fmt;
use serde::{Serialize, Deserialize};
use crate::aiplan4rust::syntax::elements::Ident;

/// Represents a type in a planning problem IR.
///
/// A type is always represented as a non-empty list of atomic type identifiers.
/// If the list contains a single identifier, it represents an atomic (primitive) type.
/// If it contains multiple identifiers, it represents a union (called `either` in PDDL) of types.
///
/// This structure allows easy representation of both simple and union types
/// while keeping the internal model flat and efficient.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Type {
    /// Non-empty list of atomic type identifiers.
    pub members: Vec<Ident>,
}

impl Type {
    /// Creates a new atomic type from a single atomic type identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the atomic type.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing an atomic type.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::ir::{Type, Ident};
    /// let t = Type::atomic_type(Ident(1));
    /// assert!(t.is_atomic_type());
    /// ```
    pub fn atomic_type(id: Ident) -> Self {
        Type { members: vec![id] }
    }

    /// Creates a new union type (either) from multiple atomic type identifiers.
    ///
    /// # Arguments
    ///
    /// * `ids` - A non-empty vector of atomic type identifiers to union.
    ///
    /// # Panics
    ///
    /// Panics if `ids` is empty.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing a union of atomic types.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::ir::{Type, Ident};
    /// let t = Type::either_type(vec![Ident(1), Ident(2)]);
    /// assert!(t.is_either_type());
    /// ```
    pub fn either_type(ids: Vec<Ident>) -> Self {
        assert!(!ids.is_empty(), "Either type must have at least one member");
        Type { members: ids }
    }

    /// Returns `true` if this type is atomic (contains exactly one member).
    pub fn is_atomic_type(&self) -> bool {
        self.members.len() == 1
    }

    /// Returns `true` if this type is a union (called `either` in PDDL) of atomic types.
    pub fn is_either_type(&self) -> bool {
        self.members.len() > 1
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_atomic_type() {
            write!(f, "t{}", self.members[0].0)
        } else {
            write!(f, "either(")?;
            for (i, id) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "t{}", id.0)?;
            }
            write!(f, ")")
        }
    }
}
