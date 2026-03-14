//! Atomic Formula Skeleton Encoding
//!
//! This module handles the encoding of atomic formula structures (predicates).
//! It specializes the generic `NamedTypedList` into an `AtomicFormulaSkeleton`,
//! representing the declaration of a predicate and its parameter signature.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{FunctionSymbolId, Type, TypeId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{ty, typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFunctionSkeleton;

/// Encodes an atomic formula skeleton from the syntax tree.
///
/// This function leverages the generic `named_typed_list` encoder to extract
/// the predicate symbol and its typed parameters, then wraps them into an
/// `AtomicFormulaSkeleton`.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the predicate (e.g., `(at ?r - robot ?l - location)`).
/// * `evaluator` - The symbol evaluator for typing resolution.
///
/// # Returns
///
/// * `Ok(AtomicFormulaSkeleton)` - The encoded predicate signature.
/// * `Err(LirError)` - If the name or the parameter list is malformed.
///
/// # Errors
///
/// Returns an error if the underlying `named_typed_list::encoding` fails,
/// typically due to a missing identifier or an unknown typing.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    functor_id: FunctionSymbolId, // Received from parent (already resolved)
    return_type: Type<TypeId>,
) -> Result<AtomicFunctionSkeleton, LirError> {
    // 1. Development safety assertions
    debug_assert!(functor_id.as_usize() != usize::MAX, "The provided functor ID is invalid.");

    // Clear variables registry to ensure a clean scope for this function skeleton
    registry.clear_variables();

    let node = subtree.node();
    let ast = subtree.tree();

    // 2. Encoding parameters (second child: index 1)
    // Following grammar flattening, AtomicFunctionSkeleton nodes now
    // consistently hold the variable list at index 1.
    let params_node_id = node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry
    )?;

    // 3. Construction of the final skeleton
    // We bind the semantic functor ID with its encoded parameters and return type.
    // Variable symbols captured during 'encode_variable_list' are attached to the skeleton.
    let variables_symbols = registry.get_variable_symbols();
    let function = AtomicFunctionSkeleton::new(
        functor_id,
        parameters,
        return_type
    ).with_variable_symbols(variables_symbols);

    Ok(function)
}
