use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a ground type in a PDDL domain.
///
/// A `Ty` defines a type of objects in the domain, optionally with parent types
/// (`super_types`) and a list of objects (`objects`) of this type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ty {
    /// Symbolic identifier of the type (e.g., "truck", "location").
    symbol: usize,

    /// Indices of parent types (supertypes).
    super_types: Vec<usize>,

    /// Indices of objects belonging to this type.
    objects: Vec<usize>,
}

impl Ty {
    /// Creates a new `Ty` with the specified `symbol`, `super_types`, and `objects`.
    ///
    /// # Parameters
    /// - `symbol`: Symbolic identifier for the type.
    /// - `super_types`: Vector of indices representing the parent types.
    /// - `objects`: Vector of indices representing objects of this type.
    ///
    /// # Returns
    /// A new instance of `Ty`.
    ///
    /// # Example
    /// ```
    /// let t = Ty::new(1, vec![0], vec![10, 11]);
    /// ```
    pub fn new(symbol: usize, super_types: Vec<usize>, objects: Vec<usize>) -> Self {
        Self {
            symbol,
            super_types,
            objects,
        }
    }

    // ----- Getters -----

    /// Returns the symbolic identifier of the type.
    ///
    /// # Returns
    /// The `symbol` as `usize`.
    pub fn symbol(&self) -> usize {
        self.symbol
    }

    /// Returns a reference to the parent type indices.
    ///
    /// # Returns
    /// Reference to a `Vec<usize>` containing the indices of super types.
    pub fn super_types(&self) -> &Vec<usize> {
        &self.super_types
    }

    /// Returns a mutable reference to the parent type indices.
    ///
    /// # Returns
    /// Mutable reference to a `Vec<usize>` containing the indices of super types.
    pub fn super_types_mut(&mut self) -> &mut Vec<usize> {
        &mut self.super_types
    }

    /// Returns a reference to the object indices of this type.
    ///
    /// # Returns
    /// Reference to a `Vec<usize>` containing object indices.
    pub fn objects(&self) -> &Vec<usize> {
        &self.objects
    }

    /// Returns a mutable reference to the object indices of this type.
    ///
    /// # Returns
    /// Mutable reference to a `Vec<usize>` containing object indices.
    pub fn objects_mut(&mut self) -> &mut Vec<usize> {
        &mut self.objects
    }

    // ----- Setters -----

    /// Sets the symbolic identifier of the type.
    ///
    /// # Parameters
    /// - `symbol`: New symbolic identifier.
    pub fn set_symbol(&mut self, symbol: usize) {
        self.symbol = symbol;
    }

    /// Sets the parent type indices.
    ///
    /// # Parameters
    /// - `super_types`: New vector of indices for parent types.
    pub fn set_super_types(&mut self, super_types: Vec<usize>) {
        self.super_types = super_types;
    }

    /// Sets the object indices for this type.
    ///
    /// # Parameters
    /// - `objects`: New vector of object indices.
    pub fn set_objects(&mut self, objects: Vec<usize>) {
        self.objects = objects;
    }
}

impl fmt::Display for Ty {
    /// Formats the type in a compact PDDL-friendly style:
    /// `symbol - super1 super2 ... : obj1 obj2 ...`
    ///
    /// # Parameters
    /// - `f`: The formatter to write to.
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure.
    ///
    /// # Example
    /// ```
    /// let t = Ty::new(1, vec![0,2], vec![10,11]);
    /// println!("{}", t); // Prints: 1 - 0 2 : 10 11
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let supers = if !self.super_types.is_empty() {
            format!("- {}", self.super_types.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" "))
        } else {
            String::new()
        };

        let objs = if !self.objects.is_empty() {
            format!(": {}", self.objects.iter()
                .map(|o| o.to_string())
                .collect::<Vec<_>>()
                .join(" "))
        } else {
            String::new()
        };

        write!(f, "{} {}{}", self.symbol, supers, objs)
    }
}
