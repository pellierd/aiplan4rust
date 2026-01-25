//! Atomic Formula Encoding
//!
//! This module handles the encoding of atomic formulas (predicates applied to terms).
//! In the LIR, a Formula represents the declaration or the occurrence of a
//! predicate with its associated parameters.

use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::named_typed_list;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;

/// Encodes an atomic formula from the syntax tree into the LIR.
///
/// This function extracts the predicate identifier and its typed parameters.
/// It is used for both predicate declarations in the domain and atomic
/// propositions in logical expressions.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the atomic formula (e.g., `(at ?robot ?location)`).
///
/// # Returns
///
/// * `Ok(Formula)` - The encoded atomic formula skeleton.
/// * `Err(LirError)` - If the identifier is missing or the parameter list is malformed.
///
/// # Errors
///
/// This function returns an error if `named_typed_list::encode` fails to
/// parse the mandatory identifier (Child 0) or the parameter list (Child 1).
pub fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<AtomicFormulaSkeleton, LirError> {
    // 1. Encode the signature (predicate name + terms/parameters)
    let header = named_typed_list::encode(subtree)?;

    // 2. Use the internal constructor to take ownership of the header
    Ok(AtomicFormulaSkeleton::from_header(header))
}
