//! Atomic Formula Skeleton Encoding
//!
//! This module handles the encoding of atomic formula structures (predicates).
//! It specializes the generic `NamedTypedList` into an `AtomicFormulaSkeleton`,
//! representing the declaration of a predicate and its parameter signature.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::FunctorID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{ty, typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFunctionSkeleton, NamedTypedList};

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
    functor_id: FunctorID, // Reçu du parent (déjà résolu)
) -> Result<AtomicFunctionSkeleton, LirError> {
    // 1. Assertions de sécurité pour le développement
    debug_assert!(functor_id.as_usize() != usize::MAX, "L'ID du functor passé est invalide.");

    registry.clear_variables();

    let node = subtree.node();
    let ast = subtree.tree();

    // 2. Encodage des paramètres (second enfant : index 1)
    let params_node_id = node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry
    )?;

    // 3. Encodage du type de retour (troisième enfant : index 2)
    let return_type_node_id = node.try_child(2)?;
    let return_type_node = ast.try_node(return_type_node_id)?;
    let return_type = ty::encode(
        &SyntaxSubtree::new(return_type_node, return_type_node_id, ast),
        registry
    )?;

    // 4. Construction du squelette final
    // On utilise NamedTypedList pour lier l'ID sémantique aux paramètres
    let variables_symbols = registry.get_variable_symbols();
    let function = AtomicFunctionSkeleton::new(
        functor_id,
        parameters,
        return_type
    )
        .with_variable_symbols(variables_symbols);
    Ok(function)
}
