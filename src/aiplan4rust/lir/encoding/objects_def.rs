//! PDDL Object Definitions Encoding
//!
//! Ce module encode les symboles globaux dans le LIR. Il gère la distinction
//! entre les objets locaux au problème et les constantes partagées du domaine.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::encoding::{typed_symbol, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::origin::Origin;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encode une liste d'objets (typiquement la section :objects du PDDL).
///
/// Note : On ne passe pas le ExprBuilder ici car les objets sont enregistrés
/// dans le `registry` et le `ir`, pas directement dans le old d'expressions.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    // Phase 1 : Enregistrement des noms et filtrage (Shared vs Local)
    let object_nodes_ids = collect_object_ids(subtree, registry, ir)?;

    // Phase 2 : Résolution des types et finalisation des définitions
    encode_definitions(subtree, registry, ir, object_nodes_ids)?;

    Ok(())
}

fn collect_object_ids(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<Vec<NodeId>, EncodingError> {
    let tree = subtree.tree();
    // Accès au premier enfant qui est la liste typée
    let list_node = tree.try_node(subtree.node().try_child(0)?)?;
    let mut object_nodes_ids = Vec::new();

    for typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(*typed_symbol_id)?;
        let symbol_node_id = typed_symbol_node.try_child(0)?;
        let symbol_node = tree.try_node(symbol_node_id)?;
        let symbol_id = symbol_node.try_ident()?;

        // Récupération de la déclaration via la table des symboles (Linker)
        let decl = registry
            .symbol_table()
            .try_get_declaration(symbol_node_id)?;

        // --- LOGIQUE DE FILTRAGE ---
        if decl.origin() == Origin::Shared {
            // C'est une constante du domaine. On l'enregistre dans le registry
            // pour la résolution locale, mais on ne l'ajoute pas aux objets du problème.
            registry.register_object_symbol(symbol_id, symbol_node_id);
            continue;
        }

        // C'est un objet propre au problème.
        registry.register_object_symbol(symbol_id, symbol_node_id);
        ir.add_object_symbol(symbol_id);

        // On l'ajoute à la liste pour le typage en Phase 2
        object_nodes_ids.push(*typed_symbol_id);
    }

    Ok(object_nodes_ids)
}

fn encode_definitions(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    object_nodes_ids: Vec<NodeId>,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();

    for typed_symbol_id in object_nodes_ids {
        let typed_symbol_node = tree.try_node(typed_symbol_id)?;
        let child_subtree = SyntaxSubtree::new(typed_symbol_node, typed_symbol_id, tree);

        // Résolution du TypedSymbol<ObjectID, TypeID>
        // Le module typed_symbol utilise le registry pour convertir l'identifiant en ObjectID
        let typed_object = typed_symbol::encode_typed_object(&child_subtree, registry)?;

        // Ajout de la définition complète (ID + Type) dans le problème
        ir.add_object_def(typed_object)?;
    }

    Ok(())
}
