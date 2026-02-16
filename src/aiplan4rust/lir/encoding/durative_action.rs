//! Durative Action Encoding
//!
//! This module handles the transformation of PDDL durative actions (temporal actions)
//! into the LIR. It manages the triple logic of temporal planning: duration constraints,
//! temporal conditions, and temporal effects.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{expr, typed_list};
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::durative_action::DurativeAction;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Encodes a PDDL durative action from the syntax tree into the LIR.
///
/// This function extracts the temporal action's signature and its three core
/// components: duration, conditions, and effects. It uses the `EncodingContext`
/// to ensure all temporal constraints are correctly bound to symbols.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the durative action definition.
/// * `registry` - The registry for symbol resolution.
/// * `ir` - The mutable `LiftedProblem` where the durative action is registered.
///
/// # Returns
///
/// * `Ok(DurativeAction)` - The encoded durative action ready for temporal planning.
/// * `Err(LirError)` - If the signature or any temporal expression fails to encoding.
///
/// # Errors
///
/// This function returns an error if:
/// * The action header is malformed.
/// * Any of the three mandatory temporal blocks (duration, condition, effect) are missing.
/// * The expression encoder fails to resolve symbols within these blocks.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // --- ÉTAPE 1 : Identité de l'Action Durative ---
    // On extrait le nom et on génère l'ID sémantique (utilise la même table que les actions)
    let action_name_node_id = node.try_child(0)?;
    let action_name_node = ast.try_node(action_name_node_id)?;
    let action_name_str_id = action_name_node.try_ident()?;

    // Réservation de l'ID officiel dans le Problem
    let action_symbol_id = ir.add_action_symbol(action_name_str_id);

    // --- ÉTAPE 2 : Encodage de la Signature (Paramètres) ---
    registry.clear_variables();

    // Comme pour l'action, le second fils (index 1) est obligatoirement un ParametersDef
    let parameters_def_id = node.try_child(1)?;
    let parameters_def_node = ast.try_node(parameters_def_id)?;
    debug_assert!(parameters_def_node.kind() == AstKind::ParametersDef);

    let vars_node_id = parameters_def_node.try_child(0)?;
    let vars_node = ast.try_node(vars_node_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(vars_node, vars_node_id, ast),
        registry
    )?;

    // Création du header utilisant l'ActionSymbolID
    let header = NamedTypedList::new(action_symbol_id, parameters);

    // --- ÉTAPE 3 : Encodage du Corps (Duration, Condition, Effect) ---
    // Le corps est le troisième fils (index 2)
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

    // --- ÉTAPE 4 : Stockage ---
    // On construit l'action durative et on l'ajoute au problème
    // Note: ir.add_durative_action_def doit être implémenté dans LiftedProblem
    let durative_action = DurativeAction::from_header(header, duration, condition, effect);
    ir.add_durative_action_def(durative_action);

    Ok(())
}
