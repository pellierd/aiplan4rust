//! Numeric Function Signature Encoding
//!
//! This module handles the extraction of numeric function signatures (fluents)
//! from the domain AST. It registers these functions in the LIR and maintains
//! the mapping between AST nodes and their internal LIR indices.

use std::collections::HashMap;
use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{atomic_function_skeleton, named_typed_list, ty};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes numeric function definitions (fluents) into the LIR.
///
/// This function parses function declarations, creates their corresponding LIR
/// skeletons, and populates the lookup table necessary for resolving numeric
/// expressions during the second encoding pass.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree corresponding to the `FunctionsDef` node.
/// * `ir` - The mutable lifted problem where functions are registered.
/// * `ast_func_to_lir_index` - A map populated with the mapping from AST `NodeId` to LIR function index.
///
/// # Returns
///
/// * `Ok(())` - If all function signatures were successfully registered.
/// * `Err(LirError)` - If a function definition is malformed or its type is invalid.
///
/// # Errors
///
/// This function returns an error if an `AtomicFunctionSkeleton` cannot be
/// constructed from the provided AST node (e.g., missing return type or name).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ir: &mut LiftedProblem,
    ast_func_to_lir_index: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    for &child_id in subtree.node().children() {
        let child_node = tree.try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, tree);

        // 1. Encode the function skeleton using the specialized free function
        // This handles the name, parameters, and the mandatory return type.
        let function_skeleton = atomic_function_skeleton::encode(&child_subtree)?;

        // 2. Register the skeleton in the IR
        ir.add_function(function_skeleton);

        // 3. Map the AST NodeId to the LIR index (O(1) lookup)
        let lir_index = ir.functions().len() - 1;
        ast_func_to_lir_index.insert(child_id, lir_index);
    }

    Ok(())
}
