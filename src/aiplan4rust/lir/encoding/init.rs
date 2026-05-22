//! Initial State Encoding
//!
//! Ce module gère la traduction de la section PDDL `:init`.
//! Il transforme les faits initiaux et les affectations numériques du fichier
//! problème en logique LIR stockée dans le Store.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::encoding::{expr, EncodingError, EncodingRegistry};
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprId};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encode l'état initial du problème à partir de la section AST `:init`.
///
/// Cette fonction extrait la collection de faits (ground facts) et de fluents.
/// Elle délègue l'encodage récursif au module `expr`, ce qui garantit que
/// tous les prédicats et fonctions initiaux sont correctement liés aux IDs
/// du problème (PredicateID, FunctorID) et aux objets (ObjectID).
///
/// # Arguments
///
/// * `subtree` - Le sous-arbre syntaxique correspondant au nœud `Init`.
/// * `registry` - Le registre pour la résolution des symboles (objets, prédicats).
/// * `builder` - Le builder d'expressions pour enregistrer les faits dans le Store.
///
/// # Returns
///
/// * `Ok(ExprId)` - L'identifiant de l'expression (souvent un AND global) dans le Store.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder, // Injection indispensable du builder
) -> Result<ExprId, EncodingError> {
    // 1. Accès au premier enfant du nœud Init (la liste des faits)
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

    // 2. Utilisation de l'encodeur d'expression avec le builder.
    // Contrairement à l'ancienne version, cela retourne un ExprId.
    // L'expression résultante est généralement un "And" de tous les faits initiaux.
    let init_expr_id = expr::encode(&child_subtree, registry, builder)?;

    Ok(init_expr_id)
}
