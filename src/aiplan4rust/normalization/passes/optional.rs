use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstNode};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::diagnostic::{DiagnosticManager};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::arena::{NodeId, ArenaNode, Arena};


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
        AstKind::ActionDef,
        AstKind::MethodDef,
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

        changed |= match kind {
            AstKind::ActionDef => {
                let action_def_body_id = node.try_child(2)?;
                normalize_action_def_body(action_def_body_id, arena)?
            },
            AstKind::MethodDef => {
                let method_def_body = node.try_child(2)?;
                normalize_method_def_body(method_def_body, arena)?
            },         // Placeholder
            AstKind::InitialTaskNetwork => Ok(false)?,    // Placeholder
            _ => Ok(false)?,
        };
    }

    Ok(changed)
}


// Fonction qui normalise un noeud de type ActionDefBody, modifie via arena_mut
fn normalize_method_def_body(
    id: NodeId,
    arena: &mut Arena<AstNode>,
) -> Result<bool, ParserInternalError> {
    let mut new_children = Vec::new();
    let mut changed = false;

    let method_def_body = arena.try_node(id)?;
    let children_len = method_def_body.children().len();

    match children_len {
        2 => {
            let span = method_def_body.span();
            new_children.push(method_def_body.try_child(0)?);
            new_children.push(method_def_body.try_child(1)?);
            let pre_def_id = allocate_method_precondition_def(arena, id, span.clone());
            new_children.insert(1, pre_def_id);
        }
        3 => {
            let node = arena.try_node(id)?;
            new_children = method_def_body.children().to_vec();
            changed = false;
        }
        _ => return Err(ParserInternalError::new("Too many children in ActionDefBody".to_string())),
    }
    let task_network_id = new_children[2];
    normalize_task_network_def(task_network_id, arena)?;

    let action_def_body = arena.try_node_mut(id)?;
    action_def_body.set_children(new_children);

    Ok(changed)
}


/// Fonction qui normalise un noeud de type ActionDefBody, modifie via arena_mut
/// Normalizes a task network definition node (`ActionDefBody`) in the AST by ensuring it has
/// exactly 3 children in the order: Subtasks, Orderings, Constraints.
/// Missing children are synthesized as empty optional nodes.
///
/// # Arguments
///
/// * `id` - The node ID of the `ActionDefBody`.
/// * `arena_mut` - A mutable reference to the AST arena.
///
/// # Returns
///
/// `Ok(true)` if any change was made, otherwise `Ok(false)`. Returns `Err` if the structure is invalid.
fn normalize_task_network_def(
    id: NodeId,
    arena_mut: &mut Arena<AstNode>,
) -> Result<bool, ParserInternalError> {
    let mut new_children = Vec::new();
    let mut changed = false;

    let task_network_def = arena_mut.try_node(id)?;
    let span = task_network_def.span().clone();
    let children_len = task_network_def.children().len();

    match children_len {
        0 => {
            new_children.push(allocate_optional(arena_mut, AstKind::OrderedSubtaskDef, AstKind::And, id, span.clone())?);
            new_children.push(allocate_optional(arena_mut, AstKind::TaskOrderingConstraintDef, AstKind::And, id, span.clone())?);
            new_children.push(allocate_optional(arena_mut, AstKind::TaskLogicalConstraintDef, AstKind::Or, id, span.clone())?);
            changed = true;
        }

        1 => {
            let child_id = task_network_def.try_child(0)?;
            let kind = arena_mut.try_node(child_id)?.kind();
            match kind {
                AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef => {
                    new_children.push(child_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskOrderingConstraintDef, AstKind::And, id, span.clone())?);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskLogicalConstraintDef, AstKind::Or, id, span.clone())?);
                }
                AstKind::TaskOrderingConstraintDef => {
                    new_children.push(allocate_optional(arena_mut, AstKind::OrderedSubtaskDef, AstKind::And, id, span.clone())?);
                    new_children.push(child_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskLogicalConstraintDef, AstKind::Or, id, span.clone())?);
                }
                AstKind::TaskLogicalConstraintDef => {
                    new_children.push(allocate_optional(arena_mut, AstKind::OrderedSubtaskDef, AstKind::And, id, span.clone())?);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskOrderingConstraintDef, AstKind::And, id, span.clone())?);
                    new_children.push(child_id);
                }
                _ => return Err(ParserInternalError::new("Unexpected child node in ActionDefBody".into())),
            }
            changed = true;
        }

        2 => {
            let first_id = task_network_def.try_child(0)?;
            let second_id = task_network_def.try_child(1)?;

            let first = arena_mut.try_node(first_id)?;
            let second = arena_mut.try_node(second_id)?;

            let first_kind = first.kind();
            let second_kind = second.kind();

            match (first_kind, second_kind) {
                (AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef, AstKind::TaskOrderingConstraintDef) => {
                    new_children.push(first_id);
                    new_children.push(second_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskLogicalConstraintDef, AstKind::Or, id, span.clone())?);
                }
                (AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef, AstKind::TaskLogicalConstraintDef) => {
                    new_children.push(first_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskOrderingConstraintDef, AstKind::And, id, span.clone())?);
                    new_children.push(second_id);
                }
                (AstKind::TaskOrderingConstraintDef, AstKind::TaskLogicalConstraintDef) => {
                    new_children.push(allocate_optional(arena_mut, AstKind::OrderedSubtaskDef, AstKind::And, id, span.clone())?);
                    new_children.push(first_id);
                    new_children.push(second_id);
                }
                (AstKind::TaskLogicalConstraintDef, AstKind::TaskOrderingConstraintDef) => {
                    new_children.push(allocate_optional(arena_mut, AstKind::OrderedSubtaskDef, AstKind::And, id, span.clone())?);
                    new_children.push(second_id);
                    new_children.push(first_id);
                }
                (AstKind::TaskOrderingConstraintDef, AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef) => {
                    new_children.push(second_id);
                    new_children.push(first_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskLogicalConstraintDef, AstKind::Or, id, span.clone())?);
                }
                (AstKind::TaskLogicalConstraintDef, AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef) => {
                    new_children.push(second_id);
                    new_children.push(allocate_optional(arena_mut, AstKind::TaskOrderingConstraintDef, AstKind::And, id, span.clone())?);
                    new_children.push(first_id);
                }
                _ => return Err(ParserInternalError::new("Unexpected combination of children in ActionDefBody".into())),
            }

            changed = true;
        }

        3 => {
            // Assume already normalized, no changes.
            return Ok(false);
        }

        _ => {
            return Err(ParserInternalError::new("Too many children in ActionDefBody".into()));
        }
    }

    let node_mut = arena_mut.try_node_mut(id)?;
    node_mut.set_children(new_children);

    Ok(changed)
}



// Fonction qui normalise un noeud de type ActionDefBody, modifie via arena_mut
fn normalize_action_def_body(
    id: NodeId,
    arena_mut: &mut Arena<AstNode>,
) -> Result<bool, ParserInternalError> {
    let mut new_children = Vec::new();
    let mut changed = false;

    let action_def_body = arena_mut.try_node(id)?;
    let children_len = action_def_body.children().len();
    let span = action_def_body.span().clone();

    match children_len {
        0 => {
            let pre_def_id = allocate_action_precondition_def(arena_mut, id, span.clone());
            let eff_def_id = allocate_effect_def(arena_mut, id, span.clone());
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
                    let eff_def_id = allocate_effect_def(arena_mut, id, span.clone());
                    new_children.push(eff_def_id);
                    changed = true;
                }
                AstKind::EffectDef => {
                    let pre_def_id = allocate_action_precondition_def(arena_mut, id, span.clone());
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




fn allocate_action_precondition_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> NodeId {
    let pre_def_id = arena.alloc(AstNode::new(AstKind::PreconditionDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let pre_id = arena.alloc(AstNode::new(AstKind::Or, AstContent::None, vec![], span, Some(pre_def_id)));
    arena.try_node_mut(pre_def_id).unwrap().set_children(vec![pre_id]);
    pre_def_id
}

fn allocate_method_precondition_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> NodeId {
    let pre_def_id = arena.alloc(AstNode::new(AstKind::MethodPreconditionDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let pre_id = arena.alloc(AstNode::new(AstKind::Or, AstContent::None, vec![], span, Some(pre_def_id)));
    arena.try_node_mut(pre_def_id).unwrap().set_children(vec![pre_id]);
    pre_def_id
}

fn allocate_effect_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> NodeId {
    let eff_def_id = arena.alloc(AstNode::new(AstKind::EffectDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let eff_id = arena.alloc(AstNode::new(AstKind::And, AstContent::None, vec![], span, Some(eff_def_id)));
    arena.try_node_mut(eff_def_id).unwrap().set_children(vec![eff_id]);
    eff_def_id
}
fn allocate_ordered_subtask_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> Result<NodeId, ParserInternalError> {
    let ordered_def_id = arena.alloc(AstNode::new(AstKind::OrderedSubtaskDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let tasks_id = arena.alloc(AstNode::new(AstKind::And, AstContent::None, vec![], span, Some(ordered_def_id)));
    arena.try_node_mut(ordered_def_id)?.set_children(vec![tasks_id]);
    Ok(ordered_def_id)
}

fn allocate_task_ordering_constraints_def(arena: &mut Arena<AstNode>, parent_id: NodeId, span: Span) -> Result<NodeId, ParserInternalError> {
    let ordered_def_id = arena.alloc(AstNode::new(AstKind::TaskOrderingConstraintDef, AstContent::None, vec![], span.clone(), Some(parent_id)));
    let tasks_id = arena.alloc(AstNode::new(AstKind::And, AstContent::None, vec![], span, Some(ordered_def_id)));
    arena.try_node_mut(ordered_def_id)?.set_children(vec![tasks_id]);
    Ok(ordered_def_id)
}

fn allocate_optional(
    arena: &mut Arena<AstNode>,
    parent_kind: AstKind,
    child_kind: AstKind,
    parent_id: NodeId,
    span: Span,
) -> Result<NodeId, ParserInternalError> {
    let parent_node_id = arena.alloc(AstNode::new(
        parent_kind,
        AstContent::None,
        vec![],
        span.clone(),
        Some(parent_id),
    ));

    let child_node_id = arena.alloc(AstNode::new(
        child_kind,
        AstContent::None,
        vec![],
        span,
        Some(parent_node_id),
    ));

    arena.try_node_mut(parent_node_id)?.set_children(vec![child_node_id]);

    Ok(parent_node_id)
}
