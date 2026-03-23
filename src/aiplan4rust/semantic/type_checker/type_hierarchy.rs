//! Type hierarchy representation for PDDL-like domains.
//!
//! This module provides the [`TypeHierarchy`] structure, which stores the relationships
//! between symbols (types) as defined in a planning domain. It is designed to support
//! both upward traversal (finding parents) and downward traversal (finding children)
//! with constant time complexity.
//!
//! # Architecture
//! The hierarchy is implemented as a bidirectional adjacency list using [`HashMap`]
//! and [`SymbolId`]. This allows the [`TypeChecker`](crate::semantic::type_checker::TypeChecker)
//! to perform transitive closure computations and subtype validations efficiently.

use crate::aiplan4rust::lang::SymbolId;
use std::collections::HashMap;
use std::fmt;

/// Represents the raw bidirectional mapping of the type hierarchy.
///
/// This structure serves as the core data provider for the [`TypeChecker`]. It stores
/// the relationships between types as declared in the PDDL domain (e.g., `truck - vehicle`).
///
/// The hierarchy is stored as an adjacency list in both directions to allow for
/// efficient traversal whether ascending (to find supertypes) or descending (to find subtypes).
#[derive(Debug, Clone, Default)]
pub struct TypeHierarchy {
    /// Maps a child type to its immediate parent types.
    ///
    /// This reflects the direct declarations in the PDDL domain.
    /// Example: `truck` -> `vec!["vehicle"]`.
    child_to_parents: HashMap<SymbolId, Vec<SymbolId>>,

    /// Maps a parent type to its immediate children.
    ///
    /// This is an inverted index computed to allow fast downward lookups
    /// in the type tree or graph.
    /// Example: `vehicle` -> `vec!["truck", "airplane"]`.
    parent_to_children: HashMap<SymbolId, Vec<SymbolId>>,
}

impl TypeHierarchy {
    /// Creates a new `TypeHierarchy` from a child-to-parents mapping.
    ///
    /// This constructor automatically computes the inverse index (`parent_to_children`)
    /// to allow $O(1)$ lookups in both directions.
    ///
    /// # Arguments
    /// * `map` - A [`HashMap`] where each key is a subtype and each value is a vector
    ///   of its immediate supertypes.
    pub fn new(map: HashMap<SymbolId, Vec<SymbolId>>) -> Self {
        let mut parent_to_children: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();

        for (child, parents) in &map {
            for &parent in parents {
                parent_to_children.entry(parent).or_default().push(*child);
            }
        }

        Self {
            child_to_parents: map,
            parent_to_children,
        }
    }

    /// Returns the immediate parents of a given type.
    ///
    /// # Example
    /// If `truck - vehicle` is defined, `get_parents(truck_id)` returns `&[vehicle_id]`.
    ///
    /// # Returns
    /// A slice of [`SymbolId`] representing the direct supertypes, or an empty slice if none.
    pub fn get_parents(&self, type_id: SymbolId) -> &[SymbolId] {
        self.child_to_parents
            .get(&type_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the immediate children of a given type.
    ///
    /// # Example
    /// If `truck - vehicle` and `car - vehicle` are defined, `get_children(vehicle_id)`
    /// returns `&[truck_id, car_id]`.
    pub fn get_children(&self, type_id: SymbolId) -> &[SymbolId] {
        self.parent_to_children
            .get(&type_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Checks if the given type acts as a parent (i.e., it has at least one subtype).
    ///
    /// Useful for identifying abstract types or branch nodes in the hierarchy.
    pub fn is_parent(&self, type_id: SymbolId) -> bool {
        self.parent_to_children.contains_key(&type_id)
    }

    /// Checks if the given type is a "leaf" (i.e., it has no subtypes).
    ///
    /// In PDDL, leaf types often represent the most specific categories
    /// to which objects can belong.
    pub fn is_leaf(&self, type_id: SymbolId) -> bool {
        !self.is_parent(type_id)
    }

    /// Returns true if the given SymbolId is declared as a parent of at least one other type.
    ///
    /// In PDDL, this identifies "super-types" (e.g., 'vehicle' in 'truck - vehicle').
    pub fn is_type_used_as_parent(&self, type_id: SymbolId) -> bool {
        // On vérifie simplement si l'ID existe dans l'index inverse.
        // Si oui, c'est qu'il a au moins un enfant.
        self.parent_to_children.contains_key(&type_id)
    }
}

impl fmt::Display for TypeHierarchy {
    /// Formats the type hierarchy for user-friendly display and debugging.
    ///
    /// This implementation prints the hierarchy as a sorted table of relationships
    /// (Child -> [Parents]). It ensures deterministic output by sorting the
    /// [`SymbolId`]s before rendering.
    ///
    /// # Example Output
    /// ```text
    /// Type Hierarchy Table:
    /// -----------------------
    ///      10 -> [1]
    ///      11 -> [1, 5]
    ///      15 -> [10]
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Type Hierarchy Table:")?;
        writeln!(f, "-----------------------")?;

        if self.child_to_parents.is_empty() {
            return writeln!(f, "  (Empty hierarchy)");
        }

        // To ensure the output is always in the same order, we collect and sort the keys.
        // This is important because HashMap's internal order is non-deterministic.
        let mut sorted_children: Vec<_> = self.child_to_parents.keys().collect();
        sorted_children.sort();

        for child in sorted_children {
            if let Some(parents) = self.child_to_parents.get(child) {
                // We convert each SymbolId (index) to its string representation.
                let parents_str: Vec<String> = parents.iter().map(|p| p.to_string()).collect();

                // '{:>4}' provides right-alignment for up to 4 digits to keep the arrows aligned.
                writeln!(f, "  {:>4} -> [{}]", child, parents_str.join(", "))?;
            }
        }

        Ok(())
    }
}
