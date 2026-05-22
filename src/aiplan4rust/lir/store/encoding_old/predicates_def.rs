//! Predicate Definitions Encoding
//!
//! This module orchestrates the extraction of predicate signatures (skeletons)
//! from the domain AST and registers them within the LIR.
//!
//! It ensures a dual mapping in the evaluator:
//! 1. **Logical Identity**: The predicate's name node is mapped to a [`PredicateID`].
//! 2. **Structural Signature**: The same node is mapped to an [`AtomSkeletonId`].
//!
//! This precise binding allows formulas (preconditions, effects, etc.) to resolve
//! atom occurrences back to their full LIR definition and unique identity during
//! the second pass.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::store::encoding_old::{atomic_formula_skeleton, EncodingRegistry};
use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes the `:predicates` section of a PDDL domain into the LIR.
///
/// This function iterates through each predicate declaration (e.g., `(at ?r - robot)`).
/// It extracts the formal signature and performs a triple operation:
/// 1. **Storage**: Adds the signature to the [`LiftedProblem`].
/// 2. **ID Retrieval**: Obtains the newly generated [`PredicateID`] and [`AtomSkeletonId`].
/// 3. **Registration**: Binds the AST `NodeId` of the predicate symbol to these LIR IDs.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `PredicatesDef` node.
/// * `evaluator` - The mutable evaluator for node-to-ID mapping.
/// * `ir` - The mutable Lifted Problem storage.
///
/// # Returns
///
/// * `Ok(())` - If all predicate signatures were successfully encoded and registered.
/// * `Err(LirError)` - If a predicate structure is invalid or typing resolution fails.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem, // On utilise le typing Problem tel que défini dans ton fichier
) -> Result<(), LirError> {
    let tree = subtree.tree();

    // On itère sur chaque déclaration de prédicat (ex: (at ?r - robot ...))
    for &atom_node_id in subtree.node().children() {
        let atom_node = tree.try_node(atom_node_id)?;

        // 1. On extrait le nom (StringID) depuis l'identifiant dans l'AST
        let name_node_id = atom_node.try_child(0)?;
        let name_str_id = tree.try_node(name_node_id)?.try_ident()?;

        // 2. IDENTITÉ : On réserve l'ID numérique du symbole
        // Utilise la nouvelle méthode publique qu'on a ajoutée
        let predicate_id = ir.add_predicate_symbol(name_str_id);

        // 3. ENCODE : On construit le squelette avec cet ID
        // Le squelette contiendra le PredicateID et les types des paramètres
        let atom_subtree = SyntaxSubtree::new(atom_node, atom_node_id, tree);
        let atom_skeleton = atomic_formula_skeleton::encode(&atom_subtree, registry, predicate_id)?;

        // 4. STOCKAGE : On enregistre la structure complète
        // Utilise la version simplifiée de add_predicate_def qui ne renvoie que l'AtomSkeletonID
        let skeleton_id = ir.add_predicate_def(atom_skeleton);

        // 5. MAPPING : On lie le NodeId de l'AST aux IDs du LIR pour la phase 2
        registry.register_atom_skeleton(name_node_id, skeleton_id);
        registry.register_predicate(name_node_id, predicate_id);
    }

    Ok(())
}
