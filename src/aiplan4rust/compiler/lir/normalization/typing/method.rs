//! # HTN Method Normalization
//!
//! This module implements the type resolution and normalization logic for HTN Methods
//! within the Lifted IR (LIR).
//!
//! ## Overview
//! A Method in a Hierarchical Task Network (HTN) defines the decomposition of an abstract
//! task into a sub-network of tasks. To ensure the grounder can process these
//! decompositions efficiently, this pass unifies all composite type signatures
//! within the method's scope into atomic identifiers.
//!
//! The transformation follows a two-step process:
//! 1. **Parameter Normalization**: Resolving types in the method's signature to ensure
//!    variable declarations use unified atomic identifiers via in-place mutation.
//! 2. **Precondition Normalization**: Traversing the precondition expression tree
//!    to resolve type signatures within logical constraints (specifically within
//!    `Exists` and `Forall` quantifiers).
//!
//! This ensures that both the "interface" (parameters) and the "logic" (preconditions)
//! of the method are perfectly aligned with the flattened type hierarchy.
//!
//! ## Memory Optimization
//! This module enforces zero runtime allocations by reusing a centralized [`Scratchpad`]
//! to execute an iterative post-order traversal over the precondition expression tree,
//! avoiding stack overflows and heap churn.

use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::typing::{expr, typed_list, TypeRegistry};
use crate::aiplan4rust::compiler::lir::problem::MethodDef;

/// Normalizes an HTN method definition in-place.
///
/// This function simplifies the type signatures within the method's parameters and
/// propagates these changes throughout the precondition expression tree hosted inside
/// the provided [`ExprStore`].
///
/// # Arguments
/// * `method` - A mutable reference to the [`MethodDef`] to be transformed.
/// * `old` - A mutable reference to the global expression old hosting the node entries.
/// * `registry` - A mutable reference to the [`TypeRegistry`] used to unify and resolve type signatures.
/// * `scratch` - An external, reusable memory arena tracking the traversal state, buffers, and ID translation cache.
///
/// # Returns
/// * `Ok(())` if both parameters and preconditions were successfully normalized.
/// * `Err(NormalizationError)` if a type resolution error or structural corruption occurs.
///
/// # Implementation Detail
/// By leveraging a mutable, pre-allocated `scratch` arena and extracting the copyable [`ExprId`]
/// before processing, this function adheres to Rust's strict aliasing rules while guaranteeing
/// an **$\mathcal{O}(1)$ dynamic allocation profile** at runtime.
pub fn normalize(
    method: &mut MethodDef,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
    scratch: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // 1. On récupère l'ID actuel des paramètres (immuable)
    let current_param_id = method.parameters();

    // 2. On passe cet ID au normalisateur de liste, avec le store pour le Hash-Consing
    let normalized_param_id =
        typed_list::normalize_typed_variable_list(current_param_id, store, registry)?;

    // 3. On met à jour la méthode avec le nouvel ID fort calculé
    method.set_parameters(normalized_param_id);

    // 4. Normalisation de l'arbre d'expression des préconditions (inchangé)
    let normalized_precondition = expr::normalize(method.precondition(), store, registry, scratch)?;
    method.set_precondition(normalized_precondition);

    Ok(())
}
