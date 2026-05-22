//! Atomic Formula Skeleton Encoding
//!
//! Ce module gère l'encodage des structures de formules atomiques (prédicats).
//! Il transforme une déclaration syntaxique en un `AtomicFormulaSkeleton`.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::PredicateSymbolId;
use crate::aiplan4rust::lir::encoding::{typed_list, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encode le squelette d'une formule atomique à partir de l'AST.
///
/// Cette fonction extrait les paramètres typés (ex: `?r - robot`) et les lie
/// à l'identifiant du prédicat pour former une signature complète.
///
/// # Arguments
///
/// * `subtree` - Le sous-arbre AST (ex: `(at ?r - robot ?l - location)`).
/// * `registry` - Le registre pour la résolution des types et variables.
/// * `predicate_id` - L'ID déjà réservé pour ce prédicat.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    predicate_id: PredicateSymbolId,
) -> Result<AtomicFormulaSkeleton, EncodingError> {
    // On nettoie les variables pour ce nouveau scope (la signature du prédicat)
    registry.clear_variables();

    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Encodage des paramètres
    // Dans l'AST d'un prédicat, les paramètres commencent souvent après le nom (index 1)
    let params_node_id = node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;

    // On utilise le module générique typed_list pour transformer l'AST en TypedList
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry,
    )?;

    // 2. Récupération des symboles de variables pour le debug/affichage
    let variable_symbols = registry.get_variable_symbols();

    // 3. Construction du squelette final
    // L'AtomicFormulaSkeleton contient l'ID du prédicat et la liste typée des paramètres
    let formula = AtomicFormulaSkeleton::new(predicate_id, parameters)
        .with_variable_symbols(variable_symbols);

    Ok(formula)
}
