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
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{atomic_function_skeleton, ty, EncodingRegistry};
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encodes function definitions into the Lifted Intermediate Representation (LIR).
///
/// This process is divided into two main phases:
/// 1. **User Definitions**: It iterates through the `:functions` block in the AST to encode
///    explicitly declared fluents (e.g., `(distance ?c1 ?c2 - city)`).
/// 2. **System Built-ins**: It injects implicit functions like `total-cost` or `total-time`
///    into the registry based on the domain's requirements (e.g., `:action-costs`).
///
/// For each user declaration, it performs the following:
/// * **Storage**: Adds the complete signature to the [`LiftedProblem`].
/// * **Identity**: Obtains the unique functor identity and [`FunctionSkeletonId`].
/// * **Mapping**: Binds the AST [`NodeId`] of the functor symbol to these LIR IDs in the registry.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `FunctionsDef` node.
/// * `registry` - The mutable registry for symbol-to-ID mapping and built-in injection.
/// * `ir` - The mutable Lifted Problem storage (LIR).
///
/// # Returns
///
/// * `Ok(())` - If all functions were successfully encoded and system built-ins were registered.
/// * `Err(LirError)` - If a definition is malformed or types are unresolved.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // --- Step 1: User-defined Functions (AST) ---
    // Encodes what is physically present in the PDDL file.
    encode_definitions(subtree, registry, ir)?;

    // --- Step 2: System Built-in Functions ---
    // Injects implicit functions that lack an AST representation.
    encode_builtin_functions(registry, ir)?;

    Ok(())
}

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
fn encode_definitions(
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

/// Injects predefined system functions (built-ins) into the encoding registry.
///
/// This step is mandatory for supporting special PDDL functions such as `total-cost`
/// (required by the `:action-costs` requirement) and `total-time` (used in temporal
/// or numeric domains).
///
/// # Architecture and LIR Consistency
///
/// Unlike user-defined functions found in the `(:functions ...)` block, these
/// system functions do not have a physical representation in the domain's AST.
/// To maintain architectural integrity:
///
/// 1. They are mapped to reserved **virtual** [`NodeId`]s (defined in [`EncodingRegistry`]).
/// 2. They are registered exclusively in the [`EncodingRegistry`] to allow symbol
///    resolution during expression encoding.
/// 3. They are **not** added to the problem definitions ([`ir.function_defs`]). This
///    prevents indexing mismatches between the LIR vectors and the AST nodes.
///
/// # Supported Requirements
///
/// * [`Requirement::ActionCosts`]: Registers the `total-cost` function.
/// * [`Requirement::Fluents`] | [`Requirement::NumericFluents`]: Registers the `total-time` function.
///
/// # Errors
///
/// Returns a [`LirError`] if the registration of a system function fails within the
/// registry (e.g., due to an unexpected internal symbol collision).
///
/// # Arguments
///
/// * `registry` - The encoding registry where system function signatures are injected.
/// * `ir` - The lifted problem, used as a read-only reference to check active requirements.
fn encode_builtin_functions(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let has_action_costs = ir.requirements().contains(&Requirement::ActionCosts);
    let has_numeric_fluents = ir.requirements().contains(&Requirement::Fluents)
        || ir.requirements().contains(&Requirement::NumericFluents);

    // Case: Action Costs -> Register 'total-cost'
    if has_action_costs {
        register_system_function(
            registry,
            ir,
            SymbolInterner::TOTAL_COST_SYMBOL_ID,
            EncodingRegistry::TOTAL_COST_NODE_ID,
            TypeId::NUMBER_TYPE_ID,
        )?;
    }

    // Case: Fluents/Numeric -> Register 'total-time'
    if has_numeric_fluents {
        register_system_function(
            registry,
            ir,
            SymbolInterner::TOTAL_TIME_SYMBOL_ID,
            EncodingRegistry::TOTAL_TIME_NODE_ID,
            TypeId::NUMBER_TYPE_ID,
        )?;
    }

    Ok(())
}

/// Injects a system-defined function into both the LIR and the encoding registry.
///
/// This helper synchronizes the creation of a built-in function by:
/// 1. Inserting the [`FunctionSymbol`] into the [`LiftedProblem`].
/// 2. Defining its [`AtomicFunctionSkeleton`] (arity 0) with the specified return type.
/// 3. Mapping both to a reserved virtual [`NodeId`] within the [`EncodingRegistry`].
///
/// This dual registration allows the encoder to resolve implicit PDDL symbols
/// (like `total-time`) as if they were standard declared functions.
///
/// # Arguments
///
/// * `registry` - The active [`EncodingRegistry`] to be updated with virtual bindings.
/// * `ir` - The [`LiftedProblem`] where the symbol and definition are stored.
/// * `symbol_id` - The unique identifier from the interner (e.g., `TOTAL_TIME_SYMBOL_ID`).
/// * `virtual_node_id` - The reserved [`NodeId`] used as a stable key for resolution.
/// * `return_type` - The [`TypeId`] of the value returned by this function (typically numeric).
///
/// # Errors
///
/// Returns a [`LirError`] if the registration process fails or if there is a
/// conflict in the registry.
fn register_system_function(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    symbol_id: SymbolId,
    virtual_node_id: NodeId,
    return_type: TypeId,
) -> Result<(), LirError> {
    // 1. Register the function symbol and its virtual mapping
    let sym = ir.add_function_symbol(symbol_id);
    registry.register_functor(virtual_node_id, sym);

    // 2. Define and register the function skeleton (signature)
    // System functions like total-time/total-cost always have an empty parameter list.
    let skeleton = AtomicFunctionSkeleton::new(
        sym,
        TypedList::empty(),
        Type::primitive(return_type),
    );

    let def_id = ir.add_function_def(skeleton);
    registry.register_function_skeleton(virtual_node_id, def_id);

    Ok(())
}
