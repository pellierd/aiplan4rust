//! Action Encoding
//!
//! This module handles the transformation of PDDL actions into the LIR.
//! It processes action signatures (parameters) and encodes their logical
//! body, including preconditions and effects, using the provided context.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::action::Action;
use crate::aiplan4rust::lir::encode::{expr, named_typed_list, EncodingRegistry};
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

    registry.clear_variables();
    // --- STEP 2: Header Encoding (Name + Parameters) ---
    // Encode the signature using the registry, which now contains the variable mappings.
    let header = named_typed_list::encode(subtree, registry)?;

    // 2. Get the body node of the action (typically Child 2)
    let def_body_node = ast.try_node(node.try_child(2)?)?;

    // Initialize precondition and effect with neutral 'and' expr by default
    let mut precondition = Expr::empty_or();
    let mut effect = Expr::empty_or();

    // 3. Iterate over the body components to extract logic blocks
    for &child_id in def_body_node.children() {
        let child_node = ast.try_node(child_id)?;
        match child_node.kind() {
            AstKind::PreconditionDef => {
                let pre_node_id = child_node.try_child(0)?;
                let pre_node = ast.try_node(pre_node_id)?;
                let pre_subtree = SyntaxSubtree::new(pre_node, pre_node_id, ast);

                // Encode the logical expression for preconditions
                precondition = expr::encode(&pre_subtree, registry)?;
            }
            AstKind::EffectDef => {
                let eff_node_id = child_node.try_child(0)?;
                let eff_node = ast.try_node(eff_node_id)?;
                let eff_subtree = SyntaxSubtree::new(eff_node, eff_node_id, ast);

                // Encode the logical expression for effects
                effect = expr::encode(&eff_subtree, registry)?;
            }
            _ => {
                // Return an error for unexpected AST nodes (e.g., :vars which is not supported here)
                return Err(LirError::action_ast_kind_error(child_node.kind()));
            }
        }
    }
    ir.add_action_def(Action::from_header(header, precondition, effect));
    Ok(())
}
