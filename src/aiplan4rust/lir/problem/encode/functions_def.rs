//! Numeric Function Signature Encoding
//!
//! This module handles the extraction of numeric function signatures (fluents)
//! from the domain AST. It registers these functions in the LIR and maintains
//! the mapping between AST nodes and their internal LIR indices.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::FunctionID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{atomic_function_skeleton, EncodingContext};
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
    context: &mut EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    for &child_id in subtree.node().children() {
        let child_node = tree.try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, tree);

        // 1. Encode the function skeleton (handles name, params, and return type)
        let function_skeleton = atomic_function_skeleton::encode(&child_subtree)?;

        // 2. Prepare the key (the Symbol)
        let functor = function_skeleton.functor();

        // 3. Register the skeleton in the IR
        ir.add_function(function_skeleton);

        // 4. Map the Symbol to the LIR index
        // Since we just added it, the ID is current length - 1
        let function_id = FunctionID::new(ir.functions().len() - 1);

        context.register_function(functor, function_id);
    }

    Ok(())
}
