//! # Initial Task Network (ITN) Flattening
//!
//! This module handles type resolution for the problem's entry point, ensuring
//! that the initial planning state is consistent with the flattened domain.
//!
//! ## Overview
//! The Initial Task Network defines the top-level variables and tasks that
//! constitute the root of the HTN decomposition process. If these variables
//! utilize composite types (e.g., `either` types), they must be resolved into
//! unified atomic types.
//!
//! This step is critical because it ensures that the search space's starting
//! point is perfectly aligned with the simplified type registry used by
//! actions, methods, and the grounding engine.

use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::passes::either_type::typed_list;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens all composite types (`Type::Either`) within an `InitialTaskNetwork` in-place.
///
/// This function remaps the type signatures of the ITN parameters to match
/// the unified atomic identifiers generated during the flattening pass.
///
/// # Parameters
/// * `itn` - A mutable reference to the [`InitialTaskNetwork`] structure to transform.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify type signatures
///   across the problem.
///
/// # Returns
/// * `Ok(())` if the initial variables were successfully flattened.
/// * `Err(LirError)` if a type signature cannot be resolved within the registry.
///
/// # Logic
/// The function focuses on the ITN's parameter list. By flattening these
/// problem-level variables, we ensure that any task call within the initial
/// network passes valid, atomic type references to the rest of the hierarchy.
pub fn flatten(
    itn: &mut InitialTaskNetwork,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // 1. Flatten the Initial Task Network parameters.
    // We resolve 'either' types into unified atomic IDs for variables declared
    // at the problem's root. This secures the "entry point" of the HTN decomposition.
    typed_list::flatten_typed_variable_list(itn.parameters_mut(), registry)
}
