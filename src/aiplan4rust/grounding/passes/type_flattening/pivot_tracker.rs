//! # Pivot Tracking and Type Deduplication
//!
//! This module implements the `PivotTracker`, the central registry for managing
//! the transformation of complex types into primitive "pivot" types.
//!
//! ## Role in the Pipeline
//! When the flattening pass encounters a complex type (like a union or `either` type),
//! it must replace it with a single, unique `TypeId`. The `PivotTracker` ensures
//! that:
//! 1. **Consistency**: The same complex type signature always maps to the same pivot ID.
//! 2. **Uniqueness**: New IDs are generated starting from a safe offset to avoid
//!    collisions with existing types.
//! 3. **Efficiency**: It prevents the explosion of redundant types by using
//!    an internal cache (`IndexMap`) for deduplication.

use std::fmt;
use indexmap::IndexMap;
use crate::aiplan4rust::lang::{Type, TypeId};

/// `PivotTracker` is a specialized registry used during the flattening process
/// to ensure that every unique set of atomic types maps to a unique, canonical `Pivot` ID.
///
/// It acts as a factory and a cache, managing the lifecycle of new type identifiers
/// created to replace hierarchical structures.
#[derive(Clone, Default, Debug)]
pub struct PivotTracker {
    /// The starting count of types in the problem.
    /// New IDs are generated as `base_len + index` to ensure they are unique
    /// within the global `TypeId` space.
    base_len: usize,

    /// Maps unique type signatures (e.g., a specific set of `either` members)
    /// to their assigned `Pivot` ID.
    /// Using [`IndexMap`] preserves insertion order, which helps with
    /// deterministic ID generation.
    pending: IndexMap<Type<TypeId>, TypeId>,
}

impl PivotTracker {
    /// Creates a new `PivotTracker` instance.
    ///
    /// # Arguments
    /// * `current_type_count` - The current number of types in the `LiftedProblem`.
    ///   New IDs will start from this value.
    pub fn new(current_type_count: usize) -> Self {
        Self {
            base_len: current_type_count,
            pending: IndexMap::new(),
        }
    }

    /// Returns the `TypeId` associated with a signature. If the signature is new,
    /// it generates and registers a new ID.
    ///
    /// # Arguments
    /// * `sig` - A slice of `TypeId` representing the resolved atomic roots of a type.
    ///
    /// # Returns
    /// * `TypeId` - The canonical ID for this specific combination of types.
    pub fn next_type_id(&mut self, sig: &[TypeId]) -> TypeId {
        // Check for existing signature without allocating a new Type object
        if let Some(&id) = self.pending.get(sig) {
            return id;
        }

        // Generate the next sequential ID
        let next_id = TypeId::from(self.base_len + self.pending.len());

        // Wrap the signature into a Type for storage
        let ty = Type::either(sig.to_vec());
        self.pending.insert(ty, next_id);

        next_id
    }

    /// Looks up if a specific signature has already been assigned a `Pivot` ID.
    ///
    /// # Arguments
    /// * `sig` - The type signature to look up.
    ///
    /// # Returns
    /// * `Option<TypeId>` - The assigned ID if found, otherwise `None`.
    pub fn get_assigned_id(&self, sig: &[TypeId]) -> Option<TypeId> {
        // IndexMap uses the comparison logic of Type to match against the slice
        self.pending.get(sig).copied()
    }

    /// Consumes the tracker and returns an iterator over the newly created pivot definitions.
    ///
    /// # Returns
    /// * `impl Iterator<Item = (Type<TypeId>, TypeId)>` - An owning iterator over the
    ///   pivots and their IDs.
    pub fn into_iter(self) -> impl Iterator<Item = (Type<TypeId>, TypeId)> {
        self.pending.into_iter()
    }

    /// Returns an iterator over references to the tracked pivot definitions.
    ///
    /// # Returns
    /// * `impl Iterator<Item = (&Type<TypeId>, &TypeId)>` - A borrowing iterator.
    pub fn iter(&self) -> impl Iterator<Item = (&Type<TypeId>, &TypeId)> {
        self.pending.iter()
    }
}

impl fmt::Display for PivotTracker {
    /// Formats the `PivotTracker` for display, showing the number of pending pivots
    /// and a detailed list of the mappings from signatures to IDs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "PivotTracker (base_len: {}, pending: {})", self.base_len, self.pending.len())?;
        for (ty, id) in &self.pending {
            // Displays each mapping: [id_1, id_2] -> TypeId(X)
            writeln!(f, "  {} -> {}", ty, id)?;
        }
        Ok(())
    }
}
