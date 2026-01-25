// lir/encode/expression.rs

use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind, ExprNode};
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
    ir: &mut LiftedProblem,
) -> Result<Expr, LirError> {
    let mut expr = Expr::new();
    let root_ast = subtree.node();

    // 1. Create root node (on passe l'ID explicitement)
    let root_id = create_and_bind_node(&mut expr, root_ast, subtree.node_id(), subtree, None, ctx, ir)?;
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
            ir
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
    _ir: &mut LiftedProblem,
) -> Result<NodeId, LirError> {
    // 1. Création basique du nœud LIR
    let kind = ExprKind::try_from(ast_node.kind())?;
    let content = ExprContent::try_from((ast_node, subtree))?;
    let node = ExprNode::new(kind, content, parent_id);

    let new_expr_node_id = expr.alloc(node);

    // 2. Gestion de la hiérarchie parent/enfant
    if let Some(pid) = parent_id {
        expr.try_node_mut(pid)?.add_child(new_expr_node_id);
    }

    // 3. Résolution sémantique (Binding) avec gestion du Kind 🎯
    if ast_node.kind() == AstKind::Predicate || ast_node.kind() == AstKind::FunctionSymbol {

        // On détermine le genre attendu pour lever l'ambiguïté (ex: SUIT prédicat vs SUIT type)
        let expected_kind = match ast_node.kind() {
            AstKind::Predicate => SymbolKind::Predicate,
            AstKind::FunctionSymbol => SymbolKind::Function,
            _ => unreachable!(),
        };

        // Appel de la résolution avec le filtre par genre
        if let Some(declaration) = ctx.symbol_table().resolve_declaration_by_usage(ast_id, expected_kind)? {
            let decl_id = declaration.node_id();

            // On cherche l'index correspondant dans notre mapping LIR
            if let Some(lir_index) = ctx.get_predicate_index(&decl_id) {
                // --- TRANSFORMATION DU CONTENT ---
                if let Ok(node) = expr.try_node_mut(new_expr_node_id) {
                    if let Some(original_ident) = node.content().as_ident() {
                        // On remplace par l'identifiant résolu
                        *node.content_mut() = ExprContent::ResolvedIdent(
                            original_ident,
                            lir_index,
                        );
                        println!("DEBUG: Nœud {} lié au symbole '{}' (Index LIR: {})",
                                 new_expr_node_id, original_ident, lir_index);
                    }
                }
            } else {
                println!("DEBUG: Échec binding pour AST:{} (Décl:{} non trouvée dans le mapping LIR)",
                         ast_id, decl_id);
            }
        }
    }

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
