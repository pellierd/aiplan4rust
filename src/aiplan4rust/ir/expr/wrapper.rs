use crate::aiplan4rust::ir::expr::{Expr, ExprNode, ExprKind, ExprContent};
use crate::aiplan4rust::tree::{NodeId, TreeArena};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::AstArenaNode;

/// Construit une `Expr` à partir d’un sous-arbre AST enraciné en `id`.
pub fn wrap(id: NodeId, ast: &TreeArena<AstArenaNode>) -> Result<Expr, ParserInternalError> {
    let mut expr = Expr::default();
    wrap_rec(id, ast, &mut expr)?;
    Ok(expr)
}

/// Fonction récursive : convertit un nœud AST en `ExprNode`, l'ajoute dans l'arène `Expr`, et retourne son ID.
fn wrap_rec(
    id: NodeId,
    ast: &TreeArena<AstArenaNode>,
    expr: &mut Expr,
) -> Result<NodeId, ParserInternalError> {
    let node = ast.try_node(id)?;

    // Convert AstKind -> ExprKind
    let kind = ExprKind::try_from(node.kind())?;

    // Convert AstContent -> ExprContent
    let content = ExprContent::try_from(node.content())?;

    // Crée le nœud de l'expression
    let expr_node = ExprNode::new(kind, content, None);
    let expr_node_id = expr.add(expr_node);

    // Recurse on children and attach them to this node
    for &child_id in node.children() {
        let child_expr_id = wrap_rec(child_id, ast, expr)?;
        expr.try_node_mut(expr_node_id)?.add_child(child_expr_id);
    }

    Ok(expr_node_id)
}
