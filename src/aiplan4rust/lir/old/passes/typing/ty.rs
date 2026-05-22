//! # Type In-Place Normalization
//!
//! This module implements the core logic for simplifying the Lifted IR (LIR) type system.
//! It ensures that every type in the problem is reduced to a single, atomic identifier.
//!
//! ## Core Logic
//! The normalization process handles two main scenarios to produce a flat type structure:
//!
//! 1. **ADL Resolution**: Identifies complex `either` types (unions) and collapses them
//!    into unique primitive identifiers managed by the [`TypeRegistry`].
//! 2. **STRIPS Rooting**: Detects empty type definitions (common in STRIPS or untyped PDDL)
//!    and maps them to the global [`ROOT_TYPE_ID`], ensuring every object has a valid parent.
//!
//! This ensures that downstream components, like the Grounder, can operate on a uniform
//! space of atomic [`TypeId`]s without handling logical unions or missing types.

use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::old::passes::typing::problem::ROOT_TYPE_ID;
use crate::aiplan4rust::lir::old::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::LirError;

/// Normalizes a type definition in-place.
///
/// This function transforms composite or empty types into a single primitive
/// representation.
///
/// # Mechanism
/// 1. **ADL Detection**: If `.is_either()` is true, the [`TypeRegistry`] resolves the
///    members into a unique, often anonymous, atomic [`TypeId`].
/// 2. **Root Detection**: If `.is_root()` is true (empty members), the type is
///    explicitly set to [`ROOT_TYPE_ID`] (the PDDL `object` type).
/// 3. **Idempotency**: If the type is already primitive and not the root, it
///    remains unchanged.
///
/// # Parameters
/// * `ty` - A mutable reference to the [`Type`] to be normalized.
/// * `registry` - The [`TypeRegistry`] used to unify signatures and manage
///   the allocation of new atomic type IDs.
///
/// # Returns
/// * `Ok(())` if the type was successfully normalized.
/// * `Err(LirError)` if an error occurs during type resolution.
pub fn normalize(ty: &mut Type<TypeId>, registry: &mut TypeRegistry) -> Result<(), LirError> {
    if ty.is_either() {
        // ADL Case: Multiple types combined.
        // Resolve the signature to a unique atomic TypeId via the registry.
        let target_id = registry.resolve(ty.members());
        *ty = Type::primitive(target_id);
    } else if ty.is_root() {
        // STRIPS Case: No types defined.
        // Map explicitly to the global ROOT_TYPE_ID (0).
        *ty = Type::primitive(ROOT_TYPE_ID);
    }
    // Primitive Case: Already has exactly one type, no action needed.

    Ok(())
}
