//! Action Encoding
//!
//! This module handles the transformation of both PDDL actions and durative actions into the LIR.
//! It processes action signatures (parameters) and encodes their logical body,
//! adapting to the specific structure of instant or temporal actions.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::action::Action;
use crate::aiplan4rust::lir::encoding::{expr, typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a PDDL action (simple or durative) from the syntax tree into the LIR.
///
/// This function extracts the action's name and parameters, then branches
/// to encode either a simple body (precondition/effect) or a durative body
/// (duration/condition/effect).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();
    let kind = node.kind();

    // --- ÉTAPE 1 : Identité de l'Action (Commune) ---
    // On extrait le nom (premier fils) et on génère l'ID sémantique
    let action_name_node_id = node.try_child(0)?;
    let action_name_node = ast.try_node(action_name_node_id)?;
    let action_name_str_id = action_name_node.try_ident()?;

    // Réservation de l'ID officiel dans le Problem
    let action_symbol_id = ir.add_action_symbol(action_name_str_id);

    // --- ÉTAPE 2 : Encodage de la Signature (Paramètres commune) ---
    registry.clear_variables();

    // Le second fils (index 1) est obligatoirement un ParametersDef
    let parameters_def_id = node.try_child(1)?;
    let parameters_def_node = ast.try_node(parameters_def_id)?;
    debug_assert!(parameters_def_node.kind() == AstKind::ParametersDef);

    let vars_node_id = parameters_def_node.try_child(0)?;
    let vars_node = ast.try_node(vars_node_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(vars_node, vars_node_id, ast),
        registry
    )?;

    // --- ÉTAPE 3 : Bifurcation selon le type d'Action ---
    match kind {
        AstKind::ActionDef => {
            // Encodage du corps d'une action simple
            let def_body_node = ast.try_node(node.try_child(2)?)?;

            let mut precondition = Expr::empty_or();
            let mut effect = Expr::empty_or();

            for &child_id in def_body_node.children() {
                let child_node = ast.try_node(child_id)?;
                match child_node.kind() {
                    AstKind::PreconditionDef => {
                        let pre_node_id = child_node.try_child(0)?;
                        let pre_node = ast.try_node(pre_node_id)?;
                        precondition = expr::encode(&SyntaxSubtree::new(pre_node, pre_node_id, ast), registry)?;
                    }
                    AstKind::EffectDef => {
                        let eff_node_id = child_node.try_child(0)?;
                        let eff_node = ast.try_node(eff_node_id)?;
                        effect = expr::encode(&SyntaxSubtree::new(eff_node, eff_node_id, ast), registry)?;
                    }
                    _ => return Err(LirError::action_ast_kind_error(child_node.kind())),
                }
            }

            let action = Action::new_snap(action_symbol_id, parameters, precondition, effect);
            ir.add_action_def(action);
        }
        AstKind::DurativeActionDef => {
            // Encodage du corps d'une action durative
            let def_body_node = ast.try_node(node.try_child(2)?)?;

            // 1. Contraintes de durée (:duration ...)
            let duration_id = def_body_node.try_child(0)?;
            let duration_node = ast.try_node(duration_id)?;
            let duration = expr::encode(&SyntaxSubtree::new(duration_node, duration_id, ast), registry)?;

            // 2. Conditions temporelles (:condition ...)
            let condition_id = def_body_node.try_child(1)?;
            let condition_node = ast.try_node(condition_id)?;
            let condition = expr::encode(&SyntaxSubtree::new(condition_node, condition_id, ast), registry)?;

            // 3. Effets temporels (:effect ...)
            let eff_node_id = def_body_node.try_child(2)?;
            let eff_node = ast.try_node(eff_node_id)?;
            let effect = expr::encode(&SyntaxSubtree::new(eff_node, eff_node_id, ast), registry)?;

            let action = Action::new_durative(action_symbol_id, parameters, duration, condition, effect);
            ir.add_action_def(action);
        }
        _ => return Err(LirError::action_ast_kind_error(kind)),
    }

    Ok(())
}
