use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a PDDL type, which can be either a primitive type (atomic)
/// or a union of primitive types (called `either` in PDDL).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Type {
    /// List of indices pointing to primitive types.
    members: Vec<usize>,
}

impl Type {
    /// Creates a new union type (`either`) from a non-empty list of type indices.
    ///
    /// # Arguments
    ///
    /// * `members` - List of indices pointing to primitive types.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing a union type.
    ///
    /// # Panics
    ///
    /// Panics if `members` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::grounding::problem::Type;
    /// let t = Type::either(vec![1, 2, 3]);
    /// assert!(t.is_either());
    /// ```
    pub fn either(members: Vec<usize>) -> Self {
        assert!(!members.is_empty(), "Either type must have at least one member");
        Self { members }
    }

    /// Creates a new primitive (atomic) type.
    ///
    /// # Arguments
    ///
    /// * `id` - The index of the atomic type.
    ///
    /// # Returns
    ///
    /// A `Type` instance representing a primitive type.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::grounding::problem::Type;
    /// let t = Type::primitive(1);
    /// assert!(t.is_primitive());
    /// ```
    pub fn primitive(id: usize) -> Self {
        Self { members: vec![id] }
    }

    /// Returns `true` if this type is primitive (contains exactly one member).
    pub fn is_primitive(&self) -> bool {
        self.members.len() == 1
    }

    /// Returns `true` if this type is a union (`either`) of atomic types.
    pub fn is_either(&self) -> bool {
        self.members.len() > 1
    }

    /// Returns a reference to the contained type indices.
    pub fn members(&self) -> &Vec<usize> {
        &self.members
    }

    /// Returns the number of type members.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns true if the type has no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns an iterator over the type indices.
    pub fn iter(&self) -> std::slice::Iter<'_, usize> {
        self.members.iter()
    }

    /// Returns a mutable iterator over the type indices.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, usize> {
        self.members.iter_mut()
    }

    /// Consumes self and returns the inner vector of type indices.
    pub fn into_vec(self) -> Vec<usize> {
        self.members
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len() == 1 {
            write!(f, "{}", self.members[0])
        } else {
            write!(f, "(either")?;
            for &t in &self.members {
                write!(f, " {}", t)?;
            }
            write!(f, ")")
        }
    }
}
