//! # Type In-Place Flattening
//!
//! This module implements the core logic for simplifying the LIR type system by
//! resolving composite type structures into unified atomic identifiers.
//!
//! ## Core Logic
//! The flattening process identifies complex type structures—specifically `either`
//! types—and collapses them into primitive types. This ensures that downstream
//! components in the planning pipeline (such as the grounder) only interact
//! with simple, non-hierarchical type identifiers.
//!
//! The mapping between a complex type's members and its new unified atomic
//! representative is managed by the [`TypeRegistry`].

use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens a type definition in-place.
///
/// If the provided [`Type`] is a composite `either` type (a union of multiple types),
/// it is replaced by a single primitive [`Type`] using a resolved [`TypeId`].
///
/// # Mechanism
/// 1. **Detection**: Checks if the type is composite using `.is_either()`.
/// 2. **Resolution**: Queries the [`TypeRegistry`] to resolve the set of member
///    types into a single unique [`TypeId`].
/// 3. **Replacement**: Updates the mutable reference in-place to a `Type::primitive`
///    wrapped around the resolved ID.
///
/// # Parameters
/// * `ty` - A mutable reference to the [`Type`] to be flattened.
/// * `registry` - The [`TypeRegistry`] used to unify signatures and manage
///   the allocation of new atomic type IDs.
///
/// # Returns
/// * `Ok(())` if the type was successfully flattened (or was already primitive).
/// * `Err(LirError)` if an error occurs during type resolution or registry access.
pub fn flatten(
    ty: &mut Type<TypeId>,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Only resolve types that represent a union of multiple types.
    if ty.is_either() {
        // Resolve the complex signature into a unique atomic ID.
        let target_id = registry.resolve(ty.members());

        // Replace the current composite Type with a primitive one.
        *ty = Type::primitive(target_id);
    }

    Ok(())
}
