//! HTN Task Network Encoding
//!
//! Transforms syntax subtrees into a structured `TaskNetwork` using an explicit builder.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TaskSkeletonId;
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::encoding::{expr, EncodingError};
use crate::aiplan4rust::lir::expr::iter::TreePreorderIter;
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprEntryKind, ExprId};
use crate::aiplan4rust::lir::problem::TaskNetwork;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Point d'entrée principal.
/// Ajout du paramètre `builder` pour la gestion des ExprId.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder,
) -> Result<TaskNetwork, EncodingError> {
    // PASS 1: Collecte des labels
    collect_task_labels(subtree, registry)?;

    // PASS 2: Encodage du contenu
    encode_task_network_content(subtree, registry, builder)
}

fn collect_task_labels(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<(), EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();

    for &child_id in node.children() {
        let child_node = ast.try_node(child_id)?;
        if matches!(
            child_node.kind(),
            AstKind::PartiallyOrderedSubtaskDef | AstKind::OrderedSubtaskDef
        ) {
            let tasks_node_id = child_node.try_child(0)?;
            let tasks_node = ast.try_node(tasks_node_id)?;

            for &tagged_task_node_id in tasks_node.children() {
                let tagged_task_node = ast.try_node(tagged_task_node_id)?;
                let tag_node_id = tagged_task_node.try_child(0)?;
                let tag_node = ast.try_node(tag_node_id)?;
                registry.register_task_label(tag_node.try_ident()?);
            }
        }
    }
    Ok(())
}

fn encode_task_network_content(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    builder: &mut ExprBuilder,
) -> Result<TaskNetwork, EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // Utilisation du builder passé en paramètre pour les valeurs par défaut
    let mut tasks_id = builder.empty_and();
    let mut ordering_id = builder.empty_and();
    let mut constraints_id = builder.empty_and();
    let mut total_ordered = false;

    for &child_id in node.children() {
        let child_node = ast.try_node(child_id)?;

        match child_node.kind() {
            AstKind::PartiallyOrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                tasks_id = expr::encode(
                    &SyntaxSubtree::new(ast.try_node(tasks_node_id)?, tasks_node_id, ast),
                    registry,
                    builder,
                )?;
            }
            AstKind::OrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                tasks_id = expr::encode(
                    &SyntaxSubtree::new(ast.try_node(tasks_node_id)?, tasks_node_id, ast),
                    registry,
                    builder,
                )?;
                total_ordered = true;
            }
            AstKind::TaskOrderingConstraintDef => {
                let ordering_node_id = child_node.try_child(0)?;
                ordering_id = expr::encode(
                    &SyntaxSubtree::new(ast.try_node(ordering_node_id)?, ordering_node_id, ast),
                    registry,
                    builder,
                )?;
            }
            AstKind::TaskLogicalConstraintDef => {
                let logical_node_id = child_node.try_child(0)?;
                constraints_id = expr::encode(
                    &SyntaxSubtree::new(ast.try_node(logical_node_id)?, logical_node_id, ast),
                    registry,
                    builder,
                )?;
            }
            _ => return Err(EncodingError::unsupported_ast_node_kind(child_node.kind())),
        }
    }

    finalize_task_network(
        tasks_id,
        ordering_id,
        constraints_id,
        total_ordered,
        registry,
        builder,
    )
}

fn finalize_task_network(
    tasks_id: ExprId,
    ordering_id: ExprId,
    constraints_id: ExprId,
    total_ordered: bool,
    registry: &EncodingRegistry,
    builder: &mut ExprBuilder,
) -> Result<TaskNetwork, EncodingError> {
    let num_tasks = registry.task_label_symbols_count();

    // On utilise désormais ExprId pour task_nodes comme convenu
    let mut task_nodes = vec![ExprId::default(); num_tasks];
    let mut task_defs = vec![TaskSkeletonId::default(); num_tasks];
    // Note: task_labels n'est plus nécessaire si tu ne t'en sers pas dans TaskNetwork::new

    // On crée l'itérateur pour parcourir l'arbre à partir de la racine 'tasks_id'
    let iter = TreePreorderIter::new(builder.store(), tasks_id);

    // Cette variable nous permet de garder en mémoire l'index du label
    // pour la prochaine tâche qu'on va croiser
    let mut current_index: Option<usize> = None;

    for (node_id, _depth, _is_last, entry) in iter {
        match entry.kind() {
            // 1. Équivalent à : if let ExprContent::TaskLabelSymbol(label_id)
            ExprEntryKind::TaskLabel(label_symbol_id) => {
                let index = label_symbol_id.as_usize();
                current_index = Some(index);
            }

            // 2. Équivalent à : if let ExprContent::TaskSkeleton(task_skeleton_id)
            ExprEntryKind::Task(skeleton_id) => {
                if let Some(index) = current_index {
                    // task_defs[index] = *task_skeleton_id
                    task_defs[index] = *skeleton_id;

                    // task_nodes[index] = task_node_id
                    // node_id est l'ExprId actuel dans le store
                    task_nodes[index] = node_id;

                    // On reset pour la tâche suivante
                    current_index = None;
                }
            }

            // LabeledTask est le parent, on laisse l'itérateur descendre
            // naturellement vers ses enfants (Label puis Task)
            ExprEntryKind::LabeledTask => {}

            _ => {}
        }
    }

    // On retourne le nouveau struct TaskNetwork (celui que tu as mis à jour)
    Ok(TaskNetwork::new(
        tasks_id,
        ordering_id,
        constraints_id,
        total_ordered,
        task_defs,
        task_nodes,
    ))
}
