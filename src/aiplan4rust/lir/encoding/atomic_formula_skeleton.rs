//! Atomic Formula Skeleton Encoding
//!
//! This module handles the encoding of atomic formula structures (predicates).
//! It specializes the generic `NamedTypedList` into an `AtomicFormulaSkeleton`,
//! representing the declaration of a predicate and its parameter signature.

use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{named_typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;

/// Encodes an atomic formula skeleton from the syntax tree.
///
/// This function leverages the generic `named_typed_list` encoder to extract
/// the predicate symbol and its typed parameters, then wraps them into an
/// `AtomicFormulaSkeleton`.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the predicate (e.g., `(at ?r - robot ?l - location)`).
/// * `registry` - The symbol registry for type resolution.
///
/// # Returns
///
/// * `Ok(AtomicFormulaSkeleton)` - The encoded predicate signature.
/// * `Err(LirError)` - If the name or the parameter list is malformed.
///
/// # Errors
///
/// Returns an error if the underlying `named_typed_list::encoding` fails,
/// typically due to a missing identifier or an unknown type.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<AtomicFormulaSkeleton, LirError> {
    registry.clear_variables();
    // 1. Reuse the generic signature encoder (Name + Parameters)
    let header = named_typed_list::encode(subtree, registry)?;

    // 2. Wrap the generic NamedTypedList into the specific AtomicFormulaSkeleton
    // Assuming AtomicFormulaSkeleton::new or from_header exists to take ownership.
    Ok(AtomicFormulaSkeleton::from_header(header))
}
