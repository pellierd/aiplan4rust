use thiserror::Error;
use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::TypeId;

/// Errors related to the construction, validation, and querying of a [`ValueRegistry`].
///
/// This enum covers failures in the type hierarchy (cycles, malformed types)
/// as well as runtime access errors (out-of-bounds indices).
#[derive(Debug, Error)]
pub enum ValueRegistryError {
    /// The provided [`TypeId`] does not exist in the registry.
    ///
    /// ### Parameters
    /// * `0`: The [`TypeId`] that was requested.
    /// * `1`: The current capacity (number of types) of the registry.
    #[error("Type ID {0:?} is out of bounds (max: {1})")]
    TypeIdOutOfBounds(TypeId, usize),

    /// A type was expected to be primitive (atomic) but is actually a complex union (either).
    ///
    /// This typically happens when trying to retrieve a contiguous memory slice for a
    /// type that hasn't been normalized or is defined as an `either` of multiple types.
    ///
    /// ### Parameters
    /// * `0`: The [`TypeId`] of the problematic type.
    /// * `1`: The number of primitive members found within this type.
    #[error("Type {0:?} must be primitive (one member), but found {1} members")]
    NotPrimitiveType(TypeId, usize),

    /// The type has no members and cannot be resolved to an object domain.
    ///
    /// This usually points to an unresolved root type or a logical error during
    /// the registry's flattening phase.
    ///
    /// ### Parameters
    /// * `0`: The [`TypeId`] of the empty type.
    #[error("Type {0:?} has no members and cannot be resolved to a domain")]
    RootType(TypeId),

    /// A circular dependency was detected in the type hierarchy.
    ///
    /// The registry requires a Directed Acyclic Graph (DAG) to flatten object domains.
    /// Cycles make it impossible to determine a finite set of inherited objects.
    ///
    /// ### Parameters
    /// * `0`: The [`TypeId`] where the cycle was first detected during traversal.
    #[error("Cycle detected in type hierarchy involving type {0:?}")]
    CycleDetected(TypeId),
}

impl ValueRegistryError {
    /// Creates a traced error for an out-of-bounds [`TypeId`].
    ///
    /// * `type_id`: The requested identifier.
    /// * `max`: The total number of types registered.
    pub fn type_out_of_bounds(type_id: TypeId, max: usize) -> Self {
        ValueRegistryError::TypeIdOutOfBounds(type_id, max).trace()
    }

    /// Creates a traced error when a type is not primitive (contains multiple members).
    ///
    /// * `type_id`: The ID of the type that failed the primitive check.
    /// * `count`: The number of members found (expected 1).
    pub fn not_primitive(type_id: TypeId, count: usize) -> Self {
        ValueRegistryError::NotPrimitiveType(type_id, count).trace()
    }

    /// Creates a traced error when a type has no members.
    ///
    /// * `type_id`: The ID of the type that is unexpectedly empty.
    pub fn root_type(type_id: TypeId) -> Self {
        ValueRegistryError::RootType(type_id).trace()
    }

    /// Creates a traced error when a cycle is detected.
    ///
    /// * `type_id`: The ID of the type involved in the circular inheritance.
    pub fn cycle_detected(type_id: TypeId) -> Self {
        ValueRegistryError::CycleDetected(type_id).trace()
    }
}

impl Traceable for ValueRegistryError {}
