//! Function Signature Encoding
//!
//! This module handles the extraction of numeric function signatures (fluents)
//! from the domain AST and registers them within the LIR.
//!
//! It ensures a dual mapping in the evaluator:
//! 1. **Functor Identity**: The function's name node is mapped to a [`StringID`] (Functor).
//! 2. **Structural Signature**: The same node is mapped to a [`FunctionSkeletonID`].
//!
//! This precise binding allows the expression encoder to resolve function calls
//! during the second encoding pass by looking up the declaration symbol's IDs
//! to validate both the fluent's identity and its expected arguments.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{atomic_function_skeleton, ty, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
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
/// * `evaluator` - The mutable evaluator for node-to-ID mapping.
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

    // The first child is the list containing our function declarations
    let typed_list_id = subtree.node().try_child(0)?;
    let typed_list_node = tree.try_node(typed_list_id)?;

    // Iterate over each item in the function list
    for &typed_item in typed_list_node.children() {
        let typed_item_node = tree.try_node(typed_item)?;

        // The function skeleton is the first child of the typed item
        let function_skeleton_node_id = typed_item_node.try_child(0)?;
        let function_skeleton_node = tree.try_node(function_skeleton_node_id)?;

        // 1. IDENTITY: Extract the StringID for the function name
        // The functor name is the first child of the skeleton node
        let functor_node_id = function_skeleton_node.try_child(0)?;
        let functor_str_id = tree.try_node(functor_node_id)?.try_ident()?;

        // 2. REGISTRATION: Reserve the FunctorID in the Problem/IR
        let functor_id = ir.add_function_symbol(functor_str_id);

        // 3. RETURN TYPE: Determine type based on the number of children in typed_item_node
        // Case 1: Only the skeleton is present -> Default to NUMBER
        // Case 2: Skeleton + explicit type -> Encode the provided type
        let return_type = match typed_item_node.children().len() {
            1 => Type::primitive(TypeId::NUMBER_TYPE_ID),
            _ => {
                // On accède à l'index 1 directement, car le validateur a dit OK
                let type_node_id = typed_item_node.try_child(1)?;
                ty::encode(&SyntaxSubtree::new(tree.try_node(type_node_id)?, type_node_id, tree), registry)?
            }
        };

        // 4. STRUCTURAL ENCODING: Build the full function skeleton
        // We delegate the parameter list encoding to the atomic_function_skeleton module
        let function_skeleton_subtree = SyntaxSubtree::new(
            function_skeleton_node,
            function_skeleton_node_id,
            tree
        );

        let function_skeleton = atomic_function_skeleton::encode(
            &function_skeleton_subtree,
            registry,
            functor_id,
            return_type,
        )?;

        // 5. STORAGE: Save the complete function definition in the IR
        let function_skeleton_id = ir.add_function_def(function_skeleton);

        // 6. MAPPING: Bind AST NodeIds to LIR IDs for cross-referencing
        registry.register_function_skeleton(functor_node_id, function_skeleton_id);
        registry.register_functor(functor_node_id, functor_id);
    }

    Ok(())
}
