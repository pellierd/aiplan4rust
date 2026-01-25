//! Durative Action Encoding
//!
//! This module handles the transformation of PDDL durative actions (temporal actions)
//! into the LIR. It manages the triple logic of temporal planning: duration constraints,
//! temporal conditions, and temporal effects.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{expr, named_typed_list};
use crate::aiplan4rust::lir::problem::encode::context::EncodingContext;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::durative_action::DurativeAction;

/// Encodes a PDDL durative action from the syntax tree into the LIR.
///
/// This function extracts the temporal action's signature and its three core
/// components: duration, conditions, and effects. It uses the `EncodingContext`
/// to ensure all temporal constraints are correctly bound to symbols.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the durative action definition.
/// * `ctx` - The encoding context for symbol resolution.
/// * `ir` - The mutable `LiftedProblem` where the durative action is registered.
///
/// # Returns
///
/// * `Ok(DurativeAction)` - The encoded durative action ready for temporal planning.
/// * `Err(LirError)` - If the signature or any temporal expression fails to encode.
///
/// # Errors
///
/// This function returns an error if:
/// * The action header is malformed.
/// * Any of the three mandatory temporal blocks (duration, condition, effect) are missing.
/// * The expression encoder fails to resolve symbols within these blocks.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<DurativeAction, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Parse the action signature (name + parameters)
    let header = named_typed_list::encode(subtree)?;

    // 2. Access the body node of the action (typically Child 2)
    let def_body_node = ast.try_node(node.try_child(2)?)?;

    // 3. Parse Duration constraints
    let duration_id = def_body_node.try_child(0)?;
    let duration_node = ast.try_node(duration_id)?;
    let duration = expr::encode(&SyntaxSubtree::new(duration_node, duration_id, ast), ctx, ir)?;

    // 4. Parse Temporal Conditions
    let condition_id = def_body_node.try_child(1)?;
    let condition_node = ast.try_node(condition_id)?;
    let condition = expr::encode(&SyntaxSubtree::new(condition_node, condition_id, ast), ctx, ir)?;

    // 5. Parse Temporal Effects
    let eff_node_id = def_body_node.try_child(2)?;
    let eff_node = ast.try_node(eff_node_id)?;
    let effect = expr::encode(&SyntaxSubtree::new(eff_node, eff_node_id, ast), ctx, ir)?;

    Ok(DurativeAction::from_header(header, duration, condition, effect))
}
