//! Action Encoding
//!
//! This module handles the transformation of PDDL actions into the LIR.
//! It processes action signatures (parameters) and encodes their logical
//! body, including preconditions and effects, using the provided context.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::action::Action;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::encoding::{expr, typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a PDDL action from the syntax tree into the LIR.
///
/// This function extracts the action's name, parameters, preconditions, and effects.
/// It uses the `EncodingContext` to bind parameters and resolve predicates
/// within the logic blocks.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the action definition.
/// * `ctx` - The encoding context for symbol resolution (parameters, predicates, etc.).
/// * `ir` - The mutable `LiftedProblem` where the action is being registered.
///
/// # Returns
///
/// * `Ok(Action)` - The fully encoded LIR action.
/// * `Err(LirError)` - If the signature or the body logic (preconditions/effects) is invalid.
///
/// # Errors
///
/// This function returns an error if:
/// * The action header (name/parameters) is malformed.
/// * An unknown or unsupported AST node is found in the action body.
/// * Expression encoding fails due to unresolved symbols.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // --- ÉTAPE 1 : Identité de l'Action ---
    // On extrait le nom (premier fils) et on génère l'ID sémantique
    let action_name_node_id = node.try_child(0)?;
    let action_name_node = ast.try_node(action_name_node_id)?;
    let action_name_str_id = action_name_node.try_ident()?;

    // Réservation de l'ID officiel dans le Problem
    let action_symbol_id = ir.add_action_symbol(action_name_str_id);

    // --- ÉTAPE 2 : Encodage de la Signature (Paramètres) ---
    registry.clear_variables();

    // Pour une action, le second fils (index 1) est obligatoirement un ParametersDef
    let parameters_def_id = node.try_child(1)?;
    let parameters_def_node = ast.try_node(parameters_def_id)?;
    debug_assert!(parameters_def_node.kind() == AstKind::ParametersDef);

    // On récupère la liste des variables (premier fils du ParametersDef)
    let vars_node_id = parameters_def_node.try_child(0)?;
    let vars_node = ast.try_node(vars_node_id)?;

    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(vars_node, vars_node_id, ast),
        registry
    )?;

    // Création du header utilisant l'ActionSymbolID
    let header = NamedTypedList::new(action_symbol_id, parameters);

    // --- ÉTAPE 3 : Encodage du Corps (Préconditions & Effets) ---
    // Le corps est généralement le troisième fils (index 2)
    let def_body_node = ast.try_node(node.try_child(2)?)?;

    let mut precondition = Expr::empty_or(); // Note: 'and' est plus neutre pour les préconditions
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
            _ => {
                return Err(LirError::action_ast_kind_error(child_node.kind()));
            }
        }
    }

    // --- ÉTAPE 4 : Stockage et Mapping ---
    // On ajoute la définition complète au Problem
    let action_skeleton = Action::from_header(header, precondition, effect);
    ir.add_action_def(action_skeleton);

    Ok(())
}
