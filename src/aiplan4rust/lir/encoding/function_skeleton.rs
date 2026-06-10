//! Atomic Function Skeleton Encoding
//!
//! Ce module gère l'encodage des structures de fonctions numériques (fluents).
//! Il transforme une déclaration syntaxique en un `AtomicFunctionSkeleton`.

use crate::aiplan4rust::lir::encoding::{typed_list, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::problem::skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::support::lang::{FunctionSymbolId, Type, TypeId};
use crate::aiplan4rust::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::syntax::ast::AstNode;

/// Encode le squelette d'une fonction numérique (fluent) à partir de l'AST.
///
/// Contrairement au prédicat, la fonction possède un type de retour
/// (ex: `number` ou un type d'objet spécifique dans certains PDDL).
///
/// # Arguments
///
/// * `subtree` - Le sous-arbre AST (ex: `(fuel-level ?t - truck)`).
/// * `registry` - Le registre pour la résolution des types et variables.
/// * `functor_id` - L'ID du symbole de fonction (déjà réservé par le parent).
/// * `return_type` - Le type de valeur retournée par cette fonction.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    functor_id: FunctionSymbolId,
    return_type: Type<TypeId>,
) -> Result<AtomicFunctionSkeleton, EncodingError> {
    // Sécurité pour le développement
    debug_assert!(
        functor_id.as_usize() != usize::MAX,
        "L'ID de functor fourni est invalide."
    );

    // On nettoie les variables pour ce nouveau scope (la signature de la fonction)
    registry.clear_variables();

    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Encodage des paramètres (ex: ?t - truck)
    // Dans l'AST aplati, la liste des variables est au deuxième enfant (index 1)
    let params_node_id = node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry,
    )?;

    // 2. Récupération des noms de variables pour le debug
    let variables_symbols = registry.get_variable_symbols();

    // 3. Construction du squelette final
    // On lie l'ID du symbole (functor), les paramètres et le type de retour.
    let function = AtomicFunctionSkeleton::new(functor_id, parameters, return_type)
        .with_variable_symbols(variables_symbols);

    Ok(function)
}
