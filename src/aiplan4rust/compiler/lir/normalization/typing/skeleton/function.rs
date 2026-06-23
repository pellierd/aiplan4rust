//! # Atomic Function Normalization
//!
//! This module implements the type resolution and normalization logic for Atomic
//! Functions (Fluents) within the Lifted IR (LIR).
//!
//! ## Overview
//! Atomic Functions map a specific set of typed parameters to a return type.
//! To maintain consistency throughout the normalized domain, both the input
//! signature and the output type must be resolved into unified atomic identifiers.
//!
//! The transformation process involves:
//! 1. **Parameter Normalization**: Resolving the types of the function's arguments
//!    using the [`typed_list`] utility via in-place mutation.
//! 2. **Return Type Normalization**: Resolving the function's result type using
//!    the support [`ty`] logic.
//!
//! This ensures that when a function is used as a term within an expression, its
//! return type is already simplified, allowing for direct value comparison and
//! grounding.

use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::typing::{ty, typed_list, TypeRegistry};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFunctionSkeleton;

/// Normalizes an atomic function skeleton in-place.
///
/// This function ensures that any composite types (e.g., `either` types) present
/// in the parameters or the return type are replaced by unified atomic [`TypeId`]s
/// managed by the [`TypeRegistry`].
///
/// # Arguments
/// * `atomic_function` - A mutable reference to the [`AtomicFunctionSkeleton`] to transform.
/// * `registry` - A mutable reference to the [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` if both the parameters and the return type were successfully normalized.
/// * `Err(NormalizationError)` if the parameter list or the return type transformation
///   encounters a registry resolution error.
///
/// # Logic
/// Functions are dual-faceted: they act as predicates in their signature but
/// as terms in their return value. This function ensures that both facets
/// are normalized, preventing type mismatches when the function is evaluated
/// within an expression tree or an effect while adhering to a strict **$\mathcal{O}(1)$ dynamic allocation profile**.
pub fn normalize(
    atomic_function: &mut AtomicFunctionSkeleton,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
) -> Result<(), NormalizationError> {
    // 1. On récupère l'ID actuel des paramètres d'entrée
    let current_param_id = atomic_function.parameters();

    // 2. On délègue au normalisateur de liste avec le store pour le Hash-Consing
    let normalized_param_id =
        typed_list::normalize_typed_variable_list(current_param_id, store, registry)?;

    // 3. On met à jour l'ID des paramètres dans le skeleton de la fonction
    atomic_function.set_parameters(normalized_param_id);

    // 4. Normalisation du type de retour (modèle non-mutable)
    let normalized_type = ty::normalize(atomic_function.ty(), registry)?;
    atomic_function.set_type(normalized_type);

    Ok(())
}
