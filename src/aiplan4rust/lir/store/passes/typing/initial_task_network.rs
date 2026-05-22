//! # Initial Task Network (ITN) Normalization
//!
//! This module handles type resolution for the problem's entry point, ensuring
//! that the initial planning state is consistent with the normalized domain.
//!
//! ## Overview
//! The Initial Task Network defines the top-level variables and tasks that
//! constitute the root of the HTN decomposition process. If these variables
//! utilize composite types (e.g., `either` types) or are untyped, they must
//! be resolved into unified atomic identifiers.
//!
//! This step is critical because it ensures that the search space's starting
//! point is perfectly aligned with the simplified type registry used by
//! actions, methods, and the grounding engine.

use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::store::passes::typing::typed_list;
use crate::aiplan4rust::lir::store::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::InitialTaskNetwork;

/// Normalizes all composite types within an `InitialTaskNetwork` in-place.
///
/// This function remaps the type signatures of the ITN parameters to match
/// the unified atomic identifiers generated during the normalization pass.
///
/// # Parameters
/// * `itn` - A mutable reference to the [`InitialTaskNetwork`] structure to transform.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify type signatures
///   across the problem.
///
/// # Returns
/// * `Ok(())` if the initial variables were successfully normalized.
/// * `Err(LirError)` if a type signature cannot be resolved within the registry.
///
/// # Logic
/// The function focuses on the ITN's parameter list. By normalizing these
/// problem-level variables, we ensure that any task call within the initial
/// network finalization valid, atomic type references to the rest of the hierarchy.
/// This secures the "entry point" of the HTN decomposition.
pub fn normalize(
    itn: &mut InitialTaskNetwork,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // 1. Normalize the Initial Task Network parameters.
    // We resolve 'either' types into unified atomic IDs for variables declared
    // at the problem's root. This guarantees that the initial state is
    // grounder-ready.
    typed_list::normalize_typed_variable_list(itn.parameters_mut(), registry)
}
