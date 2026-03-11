//! # Type In-Place Flattening
//!
//! This module provides the core logic for simplifying the LIR type system.
//!
//! ## Core Logic
//! It detects complex type structures—specifically `either` types—and 
//! collapses them into primitive types. This process ensures that the 
//! rest of the planning pipeline only has to deal with simple, 
//! non-hierarchical type identifiers.
//!
//! The mapping between the original complex members and the new 
//! primitive "pivot" is managed by the [`PivotTracker`].

use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::LirError;
use crate::type_flattening::PivotTracker;

/// Flattens a type definition in-place.
///
/// If the provided type is an `either` type (a union of multiple types), 
/// it is replaced by a single primitive type ID.
///
/// # Mechanism
/// 1. Checks if the type `is_either()`.
/// 2. Queries the [`PivotTracker`] to get or create a "pivot" `TypeId` 
///    corresponding to that specific set of member types.
/// 3. Replaces the current `Type` value with a `Type::primitive` using that ID.
///
/// # Arguments
/// * `ty` - A mutable reference to the type to be flattened.
/// * `tracker` - The stateful tracker that ensures consistent ID mapping.
///
/// # Errors
/// Returns a [`LirError`] if the tracker fails to allocate or retrieve a pivot ID.
pub fn flatten(
    ty: &mut Type<TypeId>,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    if ty.is_either() {
        // Retrieve the pivot ID (existing or newly created) from the tracker
        // based on the set of members in the 'either' type.
        let target_id = tracker.next_type_id(ty.members());

        // Replace the complex "either" structure with the corresponding 
        // primitive type. This effectively "flattens" the hierarchy.
        *ty = Type::primitive(target_id);
    }

    Ok(())
}
