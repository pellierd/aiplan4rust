//! Preference Definitions Encoding
//!
//! This module orchestrates the extraction of preference labels and their
//! associated logical conditions from the AST.
//!
//! It ensures a dual mapping in the registry:
//! 1. **Logical Identity**: The preference's label node is mapped to a [`PreferenceSymbolId`].
//! 2. **Logical Definition**: The preference's condition is encoded as an [`Expr`] and stored.

use crate::aiplan4rust::compiler::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::compiler::lir::encoding::EncodingError;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;

/// Encodes a `Preference` node from the AST into the LIR.
///
/// This function handles nodes like `(preference p1 (at robot1 roomA))`.
/// 1. **Identity Retrieval**: Registers or retrieves the [`PreferenceSymbolId`] for the label.
/// 2. **Registration**: Binds the AST `NodeId` of the label to the LIR ID.
/// 3. **Logic Encoding**: Transforms the inner condition into a LIR [`Expr`].
/// 4. **Storage**: Stores the expression in the [`LiftedProblem`] at the correct index.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `Preference` node.
/// * `registry` - The mutable registry for node-to-ID mapping.
/// * `ir` - The mutable Lifted Problem storage.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();
    let node = subtree.node();

    // 1. Extraction du label (ex: p1, p2A)
    let label_node_id = node.try_child(0)?;
    let label_node = tree.try_node(label_node_id)?;
    let label_symbol = label_node.try_ident()?;

    // 2. IDENTITÉ : On réserve l'ID et on l'ajoute au problème
    let preference_id = registry.register_preference_symbol(label_symbol);
    ir.add_preference_symbol(label_symbol);

    // 3. MAPPING : On lie le NodeId de l'AST à l'ID du LIR
    // Indispensable pour que expr::encode puisse résoudre le symbole plus tard
    registry.register_preference(label_node_id, preference_id);

    // NOTE: On n'encode PAS la logique ici (condition_expr).
    // La logique sera encodée récursivement via goal::encode ou constraints::encode
    // pendant la Phase 2 (encode_problem_logic).

    Ok(())
}
