//! PDDL Type Definition Encoding
//!
//! This module implements the two-pass encoding process for PDDL types:
//! 1. **Phase 1 (Discovery):** Scans all typing names to populate the evaluator with unique `TypeID`s.
//! 2. **Phase 2 (Definition):** Resolves inheritance relationships and adds full typing declarations to the LIR.

use crate::aiplan4rust::compiler::lir::encoding::{typed_symbol, EncodingError, EncodingRegistry};
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;
use crate::aiplan4rust::support::lang::Requirement;

/// Encodes the PDDL `:types` section into the Lifted Intermediate Representation (LIR).
///
/// This function coordinates a multi-phase process to ensure type safety and
/// support for built-in primitive types:
///
/// 1. **Symbol Collection**: Scans the AST to register all type symbols and generate
///    their unique [`TypeId`]s, allowing for forward references in the hierarchy.
/// 2. **Semantic Encoding**: Processes inheritance and properties of user-defined
///    types from the AST.
/// 3. **Built-in Injection**: Injects implicit system types (like `number`) into the
///    registry based on the domain's requirements (e.g., `:numeric-fluents` or `:action-costs`).
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `registry` - The encoding registry used to map type symbols to unique IDs.
/// * `ir` - The mutable Lifted Problem where the final typing declarations are stored.
///
/// # Returns
///
/// * `Ok(())` - If all types (user and system) were successfully encoded and registered.
/// * `Err(LirError)` - If a type definition is malformed or circular.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    // Phase 1: Register all typing symbols to generate their TypeIDs
    collect_type_ids(subtree, registry, ir)?;

    // Phase 2: Encode the semantic definitions (inheritance and properties)
    encode_definitions(subtree, registry, ir)?;

    // Phase 3: Inject system-defined types (built-ins)
    encode_builtin_types(registry, ir);

    Ok(())
}

/// Phase 1: Collects all typing identifiers (types and their supertypes)
/// and assigns them unique IDs in the evaluator.
///
/// This pass performs an initial scan of the `:types` block to register
/// every typing name before inheritance resolution occurs. It ensures
/// that types appearing only as parents (e.g., 'object' in 'number - object')
/// are assigned a `TypeID`.
///
/// This prevents "unknown typing" errors during Phase 2, especially when
/// parents are used before being defined or are implicit root types.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `evaluator` - The mutable encoding context where typing symbols are mapped to `TypeID`s.
///
/// # Returns
///
/// * `Ok(())` - If all identifiers were successfully discovered and registered.
/// * `Err(LirError)` - If the AST structure is unexpected or nodes are inaccessible.
fn collect_type_ids(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();
    let list_node = tree.try_node(subtree.node().try_child(0)?)?;

    for typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(*typed_symbol_id)?;
        let symbol_id = typed_symbol_node.try_child(0)?;
        let symbol_node = tree.try_node(symbol_id)?;
        let symbol = symbol_node.try_ident()?;
        registry.register_type_symbol(symbol, symbol_id);
        ir.add_type_symbol(symbol);

        if typed_symbol_node.children().len() > 1 {
            let type_id = typed_symbol_node.try_child(1)?;
            let type_node = tree.try_node(type_id)?;
            for &primitive_type_id in type_node.children() {
                let primitive_type_node = tree.try_node(primitive_type_id)?;
                let primitive_type_symbol = primitive_type_node.try_ident()?;
                registry.register_type_symbol(primitive_type_symbol, primitive_type_id);
                ir.add_type_symbol(primitive_type_symbol);
            }
        }
    }

    Ok(())
}

/// Phase 2: Resolves inheritance and adds full typing declarations to the LIR.
///
/// This pass iterates through the same typing list as Phase 1, but focuses on
/// semantic resolution. It uses the `typed_symbol` module to parse the
/// relationship between typing names and their parent types, then stores the
/// final `TypeDeclaration` in the LIR.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `evaluator` - The encoding context where `TypeIDs` were registered in Phase 1.
/// * `ir` - The mutable reference to the `LiftedProblem` where declarations are stored.
///
/// # Returns
///
/// * `Ok(())` - If all typing definitions were successfully resolved and added to the LIR.
/// * `Err(LirError)` - If a typing reference cannot be found in the evaluator or the AST is invalid.
///
/// # Errors
///
/// This function will return an error if:
/// * `typed_symbol::encoding` fails (e.g., a parent typing was not declared in Phase 1).
/// * The AST structure prevents navigating to the child nodes of the typing list.
fn encode_definitions(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    let tree = subtree.tree();
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = tree.try_node(list_node_id)?;

    for &typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(typed_symbol_id)?;
        let child_subtree = SyntaxSubtree::new(typed_symbol_node, typed_symbol_id, tree);

        // Encode the TypedSymbol which now can resolve its parent TypeIDs from the evaluator
        let typed_type = typed_symbol::encode_typed_type(&child_subtree, registry)?;

        // Store the final declaration in the LIR;
        ir.add_type_defs(typed_type)?;
    }

    Ok(())
}

/// Injects primitive system types (built-ins) into the registry.
///
/// The `number` type is required for any domain involving numeric fluents or
/// action costs. Note that this type is a primitive and does not have an
/// entry in the AST-based type hierarchy.
fn encode_builtin_types(registry: &mut EncodingRegistry, ir: &LiftedProblem) {
    let reqs = ir.requirements();

    // The 'number' type is required for Numeric Fluents or Action Costs.
    if reqs.contains(&Requirement::Fluents)
        || reqs.contains(&Requirement::NumericFluents)
        || reqs.contains(&Requirement::ActionCosts)
    {
        // We use the reserved virtual NodeId for the 'number' type.
        registry.register_number_type();
    }
}
