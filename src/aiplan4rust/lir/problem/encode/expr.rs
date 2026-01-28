// lir/encode/expression.rs

use crate::aiplan4rust::lang::{PredicateID, TypeID};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind, ExprNode, Resolution};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::EncodingContext;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxContent, SyntaxSubtree};

/// Encodes an AST subtree into a LIR Expression.
/// This is the "free function" version of the previous TryFrom.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
) -> Result<Expr, LirError> {
    let mut expr = Expr::new();
    let root_ast = subtree.node();

    // 1. Create root node (on passe l'ID explicitement)
    let root_id = create_and_bind_node(&mut expr, root_ast, subtree.node_id(), subtree, None, ctx)?;
    expr.set_root_id(root_id)?;

    // 2. Traversal stack: (Current AST Node, Current AST Node ID, Parent LIR Node ID)
    let mut stack: Vec<(&AstNode, NodeId, NodeId)> = Vec::new();
    push_children_to_stack(&mut stack, subtree, root_ast, root_id)?;

    // 3. Iterative traversal
    while let Some((current_ast_node, current_ast_id, parent_id)) = stack.pop() {
        let node_id = create_and_bind_node(
            &mut expr,
            current_ast_node,
            current_ast_id, // Passé ici
            subtree,
            Some(parent_id),
            ctx,
        )?;
        push_children_to_stack(&mut stack, subtree, current_ast_node, node_id)?;
    }

    Ok(expr)
}

/// Helper to create a node and immediately register its LIR binding if it's an atom.
fn create_and_bind_node(
    expr: &mut Expr,
    ast_node: &AstNode,
    ast_id: NodeId,
    subtree: &SyntaxSubtree<AstNode>,
    parent_id: Option<NodeId>,
    ctx: &EncodingContext,
) -> Result<NodeId, LirError> {
    // 1. Basic LIR node creation
    let kind = ExprKind::try_from(ast_node.kind())?;
    let content = ExprContent::try_from((ast_node, subtree))?;
    let node = ExprNode::new(kind, content, parent_id);

    let new_expr_node_id = expr.alloc(node);

    // 2. Hierarchy management
    if let Some(pid) = parent_id {
        expr.try_node_mut(pid)?.add_child(new_expr_node_id);
    }

    // 3. Mandatory Semantic Resolution
    // We only resolve nodes that are supposed to be symbols (Predicates, Functions, Parameters, etc.)
    match ast_node.kind() {
        AstKind::Predicate => {
            let decl = ctx.symbol_table()
                .try_resolve_declaration_by_usage(ast_id, SymbolKind::Predicate)?;
            let p_id = ctx.try_get_predicate_id(decl.symbol())?;
            let resolution = Resolution::Predicate(p_id);
            let node = expr.try_node_mut(new_expr_node_id)?;
            node.set_resolution(resolution);
        },
        AstKind::FunctionSymbol => {
            let decl = ctx.symbol_table()
                .try_resolve_declaration_by_usage(ast_id, SymbolKind::Function)?;
            let function = ctx.try_get_function_id(decl.symbol())?;
            let ty = ctx.try_get_type_id(decl.types().unwrap())?; // unwrape to remove
            let resolution = Resolution::Function(function, ty);
            let node = expr.try_node_mut(new_expr_node_id)?;
            node.set_resolution(resolution);

        },
        AstKind::Constant => {
            let decl = ctx.symbol_table()
                .try_resolve_declaration_by_usage(ast_id, SymbolKind::Constant)?;
            let obj = ctx.try_get_object_id(decl.symbol())?;
            let ty = ctx.try_get_type_id(decl.types().unwrap())?; // unwrape to remove
            let resolution = Resolution::Constant(obj, ty);
            let node = expr.try_node_mut(new_expr_node_id)?;
            node.set_resolution(resolution);
        },


        // Handle Parameters/Variables/Constants similarly...
        _ => {}
    };

    Ok(new_expr_node_id)
}



/// Helper to push children onto the stack, skipping meta-nodes like TypedList.
fn push_children_to_stack<'a>(
    stack: &mut Vec<(&'a AstNode, NodeId, NodeId)>,
    subtree: &'a SyntaxSubtree<'a, AstNode>,
    ast_node: &'a AstNode,
    parent_id: NodeId,
) -> Result<(), LirError> {
    for &child_id in ast_node.children().iter().rev() {
        let child_node = subtree.tree().try_node(child_id)?;

        if child_node.kind() == AstKind::TypedList {
            continue;
        }
        // On push le triplet : (Node, ID, ParentLirID)
        stack.push((child_node, child_id, parent_id));
    }
    Ok(())
}
