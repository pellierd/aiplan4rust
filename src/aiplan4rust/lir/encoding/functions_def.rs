//! Function Signature Encoding
//!
//! Ce module orchestre l'extraction des signatures de fonctions numériques (fluents)
//! depuis l'AST du domaine et les enregistre dans le LIR.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::encoding::{function_skeleton, ty, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encode la section (:functions ...) de l'AST dans le Lifted Problem (LIR).
///
/// Cette fonction extrait les noms des fonctions, leurs paramètres et leurs types
/// de retour (par défaut 'number' si non spécifié).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();

    // La liste typée des fonctions est le premier enfant du nœud (:functions ...)
    let typed_list_id = subtree.node().try_child(0)?;
    let typed_list_node = tree.try_node(typed_list_id)?;

    // On itère sur chaque déclaration de fonction
    for &typed_item in typed_list_node.children() {
        let typed_item_node = tree.try_node(typed_item)?;

        // Le squelette (nom + paramètres) est le premier enfant de l'item typé
        let function_skeleton_node_id = typed_item_node.try_child(0)?;
        let function_skeleton_node = tree.try_node(function_skeleton_node_id)?;

        // 1. IDENTITÉ : Extraction du nom de la fonction (StringID)
        let functor_node_id = function_skeleton_node.try_child(0)?;
        let functor_str_id = tree.try_node(functor_node_id)?.try_ident()?;

        // 2. RÉSERVATION : Création de l'ID du symbole (FunctorID) dans le IR
        let functor_id = ir.add_function_symbol(functor_str_id);

        // 3. TYPE DE RETOUR :
        // En PDDL, si aucun type n'est spécifié, c'est un "number" par défaut.
        let return_type = match typed_item_node.children().len() {
            1 => Type::primitive(TypeId::NUMBER_TYPE_ID),
            _ => {
                let type_node_id = typed_item_node.try_child(1)?;
                ty::encode(
                    &SyntaxSubtree::new(tree.try_node(type_node_id)?, type_node_id, tree),
                    registry,
                )?
            }
        };

        // 4. SIGNATURE : Encodage des paramètres (ex: ?v - vehicule)
        let function_skeleton_subtree =
            SyntaxSubtree::new(function_skeleton_node, function_skeleton_node_id, tree);

        let function_skeleton = function_skeleton::encode(
            &function_skeleton_subtree,
            registry,
            functor_id,
            return_type,
        )?;

        // 5. STOCKAGE : Ajout de la définition complète au LIR
        let function_skeleton_id = ir.add_function_def(function_skeleton);

        // 6. MAPPING : Enregistrement pour la Phase 2 (ExprBuilder)
        // Lie le NodeId du nom à son identité logique et sa signature structurelle.
        registry.register_function_skeleton(functor_node_id, function_skeleton_id);
        registry.register_functor(functor_node_id, functor_id);
    }

    Ok(())
}
