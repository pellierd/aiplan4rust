//! Predicate Signature Encoding
//!
//! This module handles the extraction of predicate signatures from the domain
//! AST and registers them within the LIR. It also maintains the mapping
//! between AST nodes and their corresponding LIR indices.

use crate::aiplan4rust::lang::PredicateID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{atomic_formula_skeleton, EncodingContext};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes predicate definitions into the Lifted Intermediate Representation.
///
/// This function iterates through predicate declarations, converts them into
/// LIR skeletons, and populates the lookup table used for symbol resolution
/// in subsequent encoding passes.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree containing the `PredicatesDef` node.
/// * `ir` - The mutable lifted problem where predicates are registered.
/// * `ast_node_to_predicate_declaration` - A map to be populated with the mapping from AST `NodeId` to LIR predicate index.
///
/// # Returns
///
/// * `Ok(())` - If all predicates were successfully registered and mapped.
/// * `Err(LirError)` - If a predicate definition is malformed.
///
/// # Errors
///
/// Returns an error if an `AtomicFormulaSkeleton` cannot be constructed from
/// the provided AST node.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    context: &mut EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    // On itère sur chaque définition de prédicat dans la liste
    for &child_id in subtree.node().children() {
        let child_node = tree.try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, tree);

        // 1. Encode le squelette du prédicat (nom + paramètres)
        let atomic_formula_skeleton = atomic_formula_skeleton::encode(&child_subtree)?;

        // 2. Récupère le symbole (propriété directe, pas de clone nécessaire)
        let predicate = atomic_formula_skeleton.predicate();

        // 3. Calcule l'ID avant l'ajout (index 0-based)
        let predicate_id = PredicateID::new(ir.predicates().len());

        // 4. Mappe le symbole à l'ID
        context.register_predicate(predicate, predicate_id);

        // 5. Enregistre dans le LiftedProblem
        ir.add_predicate(atomic_formula_skeleton);
    }

    Ok(())
}
