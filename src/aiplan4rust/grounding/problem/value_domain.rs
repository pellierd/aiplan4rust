use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents a set of values associated with a domain.
///
/// A `ValueDomain` is essentially a collection of indices (`usize`) representing
/// the possible values of a type, variable, object, etc.
/// It can be reused in different contexts (types, fluents, CSP variables, etc.).
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValueDomain {
    values: Vec<usize>,
}

impl ValueDomain {
    /// Creates a new empty `ValueDomain`.
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    // Creates a new empty `ValueDomain`.
    pub fn empty() -> Self {
        Self { values: Vec::new() }
    }

    /// Creates a `ValueDomain` from an existing vector of values.
    pub fn from_values(values: Vec<usize>) -> Self {
        Self { values }
    }

    /// Returns a reference to the internal vector of values.
    pub fn values(&self) -> &[usize] {
        &self.values
    }

    /// Returns an iterator over the values.
    pub fn iter(&self) -> std::slice::Iter<'_, usize> {
        self.values.iter()
    }

    /// Adds a new value to the domain.
    pub fn add(&mut self, value: usize) {
        self.values.push(value);
    }

    /// Adds all values from another domain (union operation).
    pub fn union(&mut self, other: &ValueDomain) {
        for &v in other.values.iter() {
            self.add(v);
        }
    }

    /// Returns a new `ValueDomain` containing the intersection of self and other.
    pub fn intersect(&self, other: &ValueDomain) -> ValueDomain {
        let mut result = ValueDomain::new();
        for &v in self.values.iter() {
            if other.contains(v) {
                result.add(v);
            }
        }
        result
    }

    /// Returns true if the domain contains the given value.
    pub fn contains(&self, value: usize) -> bool {
        self.values.contains(&value)
    }

    /// Returns the number of values in the domain (cardinality).
    pub fn cardinality(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the domain is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Implements pretty printing for ValueDomain
impl fmt::Display for ValueDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values_str: Vec<String> = self.values.iter().map(|v| v.to_string()).collect();
        write!(f, "{{{}}}", values_str.join(", "))
    }
}
