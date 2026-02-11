//! Function Signature Encoding
//!
//! This module handles the extraction of numeric function signatures (fluents)
//! from the domain AST and registers them within the LIR.
//!
//! It ensures a dual mapping in the registry:
//! 1. **Functor Identity**: The function's name node is mapped to a [`StringID`] (Functor).
//! 2. **Structural Signature**: The same node is mapped to a [`FunctionSkeletonID`].
//!
//! This precise binding allows the expression encoder to resolve function calls
//! during the second encoding pass by looking up the declaration symbol's IDs
//! to validate both the fluent's identity and its expected arguments.

use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encode::{atomic_function_skeleton, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes function definitions into the Lifted Intermediate Representation (LIR).
///
/// This function iterates through each child of the `:functions` node (e.g., `(total-cost) - number`).
/// It performs a triple operation for each declaration:
/// 1. **Storage**: Adds the complete signature to the [`LiftedProblem`].
/// 2. **ID Retrieval**: Obtains the unique [`StringID`] (functor identity) and [`FunctionSkeletonID`].
/// 3. **Registration**: Binds the AST `NodeId` of the functor symbol to these LIR IDs.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `FunctionsDef` node.
/// * `registry` - The mutable registry for node-to-ID mapping.
/// * `ir` - The mutable Lifted Problem storage.
///
/// # Returns
///
/// * `Ok(())` - If all functions were encoded and their functors bound to LIR IDs.
/// * `Err(LirError)` - If a definition is malformed or types are unresolved.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    // Iterate over each function definition (e.g., `(distance ?a ?b) - number`)
    for &function_skeleton_node_id in subtree.node().children() {
        let function_skeleton_node = tree.try_node(function_skeleton_node_id)?;
        let function_skeleton_subtree = SyntaxSubtree::new(
            function_skeleton_node,
            function_skeleton_node_id,
            tree
        );

        // 1. Encode the function skeleton (Functor, Parameters, and Return Type)
        // This validates types and builds the structural representation.
        let function_skeleton = atomic_function_skeleton::encode(
            &function_skeleton_subtree,
            registry
        )?;

        // 2. Identify the Functor NodeId
        // We register the ID of the symbol itself (e.g., 'distance') rather than
        // the parent expression node to match the symbol table's declaration lookup.
        let functor_node_id = function_skeleton_subtree.node().children()[0];

        // 3. Physical storage in the LIR
        // The LIR returns both the logical identity (StringID/FunctorID)
        // and the structural ID (FunctionSkeletonID).
        let (functor_id, function_skeleton_id) = ir.add_function_def(function_skeleton);

        // 4. Node mapping in the registry
        // Binds the functor's AST NodeId to both LIR identifiers.
        registry.register_function_skeleton(functor_node_id, function_skeleton_id);
        registry.register_functor(functor_node_id, functor_id);
    }

    Ok(())
}
