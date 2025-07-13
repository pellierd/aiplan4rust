
use std::collections::HashMap;
use std::collections::HashSet;
use crate::aiplan4rust::arena;
use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstNode};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::arena::{NodeId, ArenaNode, Arena};
use crate::aiplan4rust::syntax::ast::kind::Kind;

// Fonction principale qui orchestre la normalisation
pub fn normalize_optional(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut changed = false;

    // Emprunt immuable limité à la fonction collect_nodes_to_normalize
    let to_normalize = collect_nodes_to_normalize(ast);

    // Emprunt mutable pour la modification des noeuds
    let arena_mut = ast.arena_mut();
    changed = normalize_nodes(to_normalize, arena_mut)?;

    Ok(changed)
}

fn collect_nodes_to_normalize(ast: &Ast) -> Vec<NodeId> {
    let arena = ast.arena();
    arena.collect_nodes_of_kind(&[
        AstKind::ActionDefBody,
        AstKind::MethodDefBody,
        AstKind::TaskNetworkDef,
        AstKind::InitialTaskNetwork,
    ])
}

// Fonction qui normalise une liste de noeuds, sépare emprunts immuables et mutables
fn normalize_nodes(
    nodes: Vec<NodeId>,
    arena: &mut Arena<AstNode>,
) -> Result<bool, ParserInternalError> {
    let mut changed = false;

    for node_id in nodes {
        let node = arena.try_node(node_id)?;
        let kind = node.kind();
        let children_len = node.children().len();
        let span = node.span().clone();

        changed |= match kind {
            AstKind::ActionDefBody => normalize_action_def_body(node_id, children_len, span, arena)?,
            AstKind::MethodDefBody => Ok(false)?,         // Placeholder
            AstKind::TaskNetworkDef => Ok(false)?,        // Placeholder
            AstKind::InitialTaskNetwork => Ok(false)?,    // Placeholder
            _ => Ok(false)?,
        };
    }

    Ok(changed)
}

// Fonction qui normalise un noeud de type ActionDefBody, modifie via arena_mut
fn normalize_action_def_body(
    id: NodeId,
    children_len: usize,
    span: Span,
    arena_mut: &mut Arena<AstNode>,
) -> Result<bool, ParserInternalError> {
    let mut new_children = Vec::new();
    let mut changed = false;

    match children_len {
        0 => {
            let pre_def_id = create_precondition_def(arena_mut, id, span.clone());
            let eff_def_id = create_effect_def(arena_mut, id, span.clone());
            new_children.push(pre_def_id);
            new_children.push(eff_def_id);
            changed = true;
        }
        1 => {
            let node = arena_mut.try_node(id)?;
            let first_child_id = node.try_child(0)?;
            let first_child_kind = arena_mut.try_node(first_child_id)?.kind();

            match first_child_kind {
                AstKind::PreconditionDef => {
                    new_children.push(first_child_id);
                    let eff_def_id = create_effect_def(arena_mut, id, span.clone());
                    new_children.push(eff_def_id);
                    changed = true;
                }
                AstKind::EffectDef => {
                    let pre_def_id = create_precondition_def(arena_mut, id, span.clone());
                    new_children.push(pre_def_id);
                    new_children.push(first_child_id);
                    changed = true;
                }
                _ => return Err(ParserInternalError::new("Unexpected child node".to_string())),
            }
        }
        2 => {
            let node = arena_mut.try_node(id)?;
            new_children = node.children().to_vec();
            changed = false;
        }
        _ => return Err(ParserInternalError::new("Too many children in ActionDefBody".to_string())),
    }

    let action_def_body = arena_mut.try_node_mut(id)?;
    action_def_body.set_children(new_children);

    Ok(changed)
}




fn create_precondition_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> NodeId {
    let pre_def_id = arena.alloc(AstNode::new(AstKind::PreconditionDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let pre_id = arena.alloc(AstNode::new(AstKind::Or, AstContent::None, vec![], span, Some(pre_def_id)));
    arena.try_node_mut(pre_def_id).unwrap().set_children(vec![pre_id]);
    pre_def_id
}

fn create_effect_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> NodeId {
    let eff_def_id = arena.alloc(AstNode::new(AstKind::EffectDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let eff_id = arena.alloc(AstNode::new(AstKind::And, AstContent::None, vec![], span, Some(eff_def_id)));
    arena.try_node_mut(eff_def_id).unwrap().set_children(vec![eff_id]);
    eff_def_id
}
