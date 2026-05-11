//! Predicate Definitions Encoding
//!
//! Ce module orchestre l'extraction des signatures de prédicats (skeletons)
//! depuis l'AST du domaine et les enregistre dans le LIR.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::store::encoding::{atomic_skeleton, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::store::problem::NewLiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encode la section :predicates d'un domaine PDDL.
///
/// Pour chaque prédicat (ex: `(at ?r - robot)`), cette fonction effectue :
/// 1. La création d'un `PredicateID` (l'identité logique).
/// 2. L'encodage de la signature (les types des paramètres).
/// 3. L'enregistrement du mapping AST -> LIR pour permettre la résolution
///    ultérieure des formules atomiques dans le Store.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut NewLiftedProblem,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();

    // On itère sur chaque déclaration de prédicat dans la liste
    for &atom_node_id in subtree.node().children() {
        let atom_node = tree.try_node(atom_node_id)?;

        // 1. Extraction du nom (identifiant AST -> StringID)
        let name_node_id = atom_node.try_child(0)?;
        let name_str_id = tree.try_node(name_node_id)?.try_ident()?;

        // 2. RÉSERVATION : On crée l'ID unique du symbole de prédicat dans le problème
        let predicate_id = ir.add_predicate_symbol(name_str_id);

        // 3. SIGNATURE : On encode la liste des types des paramètres
        // atomic_formula_skeleton::encode va parcourir les paramètres (ex: ?r - robot)
        let atom_subtree = SyntaxSubtree::new(atom_node, atom_node_id, tree);
        let atom_skeleton = atomic_skeleton::encode(&atom_subtree, registry, predicate_id)?;

        // 4. STOCKAGE : On ajoute la définition complète au LIR
        // Cela retourne un AtomSkeletonId qui représente cette signature précise.
        let skeleton_id = ir.add_predicate_def(atom_skeleton);

        // 5. ENREGISTREMENT : Crucial pour la Phase 2 (Encodage des expressions)
        // On lie le NodeId de l'AST au PredicateID et au SkeletonID.
        // Quand le builder rencontrera "(at ...)" dans une précondition,
        // il saura quel prédicat et quelle signature utiliser.
        registry.register_atom_skeleton(name_node_id, skeleton_id);
        registry.register_predicate(name_node_id, predicate_id);
    }

    Ok(())
}
