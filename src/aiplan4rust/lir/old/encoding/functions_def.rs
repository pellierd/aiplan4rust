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
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Requirement, SymbolId, Type, TypeId, TypedList};
use crate::aiplan4rust::lir::old::encoding::{atomic_function_skeleton, ty, EncodingRegistry};
use crate::aiplan4rust::lir::old::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::old::problem::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encodes user-defined function declarations from the AST into the Lifted Problem (LIR).
///
/// This function iterates through the `(:functions ...)` block, extracts function names,
/// resolves their parameter lists, and determines their return types.
///
/// # Returns
/// * `Ok(())` if all functions were successfully encoded and registered.
/// * `Err(LirError)` if a structural error is encountered in the AST.
///
/// # Details
/// 1. **Identity**: Extracts the function name (functor) from the skeleton.
/// 2. **Return Type**: Defaults to `TypeId::NUMBER_TYPE_ID` if no explicit type is provided,
///    supporting both standard and typed fluents.
/// 3. **Storage**: Adds both the function symbol and its full definition to the [`LiftedProblem`].
/// 4. **Mapping**: Updates the [`EncodingRegistry`] to map AST [`NodeId`]s to the new LIR IDs.
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
                let type_node_id = typed_item_node.try_child(1)?;
                ty::encode(
                    &SyntaxSubtree::new(tree.try_node(type_node_id)?, type_node_id, tree),
                    registry,
                )?
            }
        };

        // 4. STRUCTURAL ENCODING: Build the full function skeleton
        let function_skeleton_subtree =
            SyntaxSubtree::new(function_skeleton_node, function_skeleton_node_id, tree);

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
