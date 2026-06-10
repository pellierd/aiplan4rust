//! Goal Condition Encoding
//!
//! Ce module gère la traduction de la section PDDL `:goal`.
//! Il transforme les exigences logiques du but en une expression LIR
//! qui doit être satisfaite dans l'état final.

use crate::aiplan4rust::compiler::lir::encoding::{expr, EncodingError, EncodingRegistry};
use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId};
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;

/// Encode la condition de but du problème à partir de la section AST `:goal`.
///
/// Cette fonction extrait la racine logique de la spécification du but et délègue
/// son encodage récursif au module `expr`. Elle garantit que tous les symboles
/// (prédicats, objets) au sein du but sont correctement résolus.
///
/// # Arguments
///
/// * `subtree` - Le sous-arbre syntaxique correspondant au nœud `Goal`.
/// * `registry` - Le registre pour la résolution des symboles.
/// * `builder` - Le builder d'expressions pour enregistrer le but dans le Store.
///
/// # Returns
///
/// * `Ok(ExprId)` - L'identifiant de l'expression de but dans le Store.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder, // Injection du builder
) -> Result<ExprId, EncodingError> {
    // 1. Accès au premier enfant du nœud Goal (la racine de l'expression logique)
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

    // 2. Encodage récursif de l'expression de but.
    // Comme pour l'init, cela retourne maintenant un ExprId pointant vers le Store.
    let goal_expr_id = expr::encode(&child_subtree, registry, builder)?;

    Ok(goal_expr_id)
}
