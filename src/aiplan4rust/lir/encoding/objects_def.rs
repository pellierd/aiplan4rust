//! PDDL Object Definitions Encoding
//!
//! This module provides the core ops for encoding global symbols into the LIR.
//! It is used directly for parsing the `:objects` section of a PDDL problem,
//! and is called by the `constants_def` module to parse domain constants.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{typed_symbol, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a list of objects from the syntax tree into the Lifted Intermediate Representation (LIR).
///
/// This function processes a typed list of symbols, resolves their types, and performs
/// the dual registration:
/// 1. **LIR Storage**: The object is added to the `LiftedProblem`.
/// 2. **Context Resolution**: The symbol's name is mapped to its new `ObjectID` in the
///    `EncodingContext` to allow future lookups (e.g., in action effects).
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the typed list (e.g., from `:objects` or `:constants`).
/// * `evaluator` - The mutable encoding context used for symbol-to-ID binding.
/// * `ir` - The mutable Lifted Problem where the objects are stored.
///
/// # Returns
///
/// * `Ok(())` - If all objects were successfully encoded and registered.
/// * `Err(LirError)` - If the AST is malformed or typing resolution fails.
///
/// # Errors
///
/// This function returns an error if:
/// * The internal AST structure for the typed list is unreachable.
/// * Any individual symbol fails to encoding (e.g., refers to a non-existent typing).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    // Phase 1: Register all object names to generate their ObjectIDs in the evaluator
    collect_object_ids(subtree, registry, ir)?;

    // Phase 2: Resolve types and finalize the object definitions in the LIR
    encode_definitions(subtree, registry, ir)?;

    Ok(())
}

/// Phase 1: Collects all object identifiers and assigns them unique IDs in the evaluator.
///
/// This ensures that even if an object is referenced elsewhere, its ID is already
/// known to the evaluator.
fn collect_object_ids(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();
    let list_node = tree.try_node(subtree.node().try_child(0)?)?;

    for typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(*typed_symbol_id)?;

        // In PDDL AST, the first child of a TypedSymbol node is the identifier (name)
        let symbol_node_id = typed_symbol_node.try_child(0)?;
        let symbol_node = tree.try_node(symbol_node_id)?;
        let symbol_id = symbol_node.try_ident()?;

        // Register the object. Your `register_object_symbol` ops handles
        // the NodeId mapping and ID generation.
        registry.register_object_symbol(symbol_id, symbol_node_id);
        ir.add_object_symbol(symbol_id);
    }
    Ok(())
}

/// Phase 2: Resolves object types and adds full declarations to the LIR.
///
/// It uses the `typed_symbol` module to resolve the `TypeID` for each object,
/// then stores the resulting `TypedSymbol<ObjectID, TypeID>` in the IR.
fn encode_definitions(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = tree.try_node(list_node_id)?;

    for &typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(typed_symbol_id)?;
        let child_subtree = SyntaxSubtree::new(typed_symbol_node, typed_symbol_id, tree);

        // 1. Encodage de la structure (ID + Type)
        let typed_object = typed_symbol::encode_typed_object(&child_subtree, registry)?;

        // 2. Ajout au LIR avec son nom
        ir.add_object_def(typed_object)?;
    }

    Ok(())
}
