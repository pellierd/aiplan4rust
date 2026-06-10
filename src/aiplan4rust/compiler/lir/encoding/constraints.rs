use crate::aiplan4rust::compiler::lir::encoding::{expr, EncodingError, EncodingRegistry};
use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId};
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;

// lir/encoding/constraints.rs

pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder, // Injection indispensable du builder
) -> Result<ExprId, EncodingError> {
    let node = subtree.node();

    // Sécurité au cas où la grammaire changerait,
    // mais selon ta def, children[0] est toujours là.
    let child_id = node.try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

    // On encode directement l'enfant (le ConGD ou PrefConGD)
    // Le LIR résultant aura pour racine le 'and', le 'always', ou la 'preference'
    expr::encode(&child_subtree, registry, builder)
}
