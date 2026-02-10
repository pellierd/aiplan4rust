//! PDDL Object Definitions Encoding
//!
//! This module provides the core logic for encoding global symbols into the LIR.
//! It is used directly for parsing the `:objects` section of a PDDL problem,
//! and is called by the `constants_def` module to parse domain constants.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::ObjectID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encode::{typed_symbol, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, Node, SyntaxSubtree, Tree};

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
/// * `registry` - The mutable encoding context used for symbol-to-ID binding.
/// * `ir` - The mutable Lifted Problem where the objects are stored.
///
/// # Returns
///
/// * `Ok(())` - If all objects were successfully encoded and registered.
/// * `Err(LirError)` - If the AST is malformed or type resolution fails.
///
/// # Errors
///
/// This function returns an error if:
/// * The internal AST structure for the typed list is unreachable.
/// * Any individual symbol fails to encode (e.g., refers to a non-existent type).
/*pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // 1. Access the list container (usually the first child of the Def node)
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    // 2. Iterate and encode each symbol in the list
    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        // Resolve the TypedSymbol structure (StringID + TypeID)
        let object = typed_symbol::encode_typed_object(&child_subtree, registry)?;
        let object_node_id = child_node.try_child(0)?;

        // Add to LIR and retrieve the definitive ObjectID
        let object_id = ir.add_object(object);

        // Bind the name to the ID in the registry for semantic lookups
        registry.register_object(object_node_id, object_id);
    }

    Ok(())
}*/

/// Encodes the PDDL `:objects` or `:constants` section into the Lifted Intermediate Representation (LIR).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    // Phase 1: Register all object names to generate their ObjectIDs in the registry
    collect_object_ids(subtree, registry, ir)?;

    // Phase 2: Resolve types and finalize the object definitions in the LIR
    encode_definitions(subtree, registry, ir)?;

    Ok(())
}

/// Phase 1: Collects all object identifiers and assigns them unique IDs in the registry.
///
/// This ensures that even if an object is referenced elsewhere, its ID is already
/// known to the registry.
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

        // Register the object. Your `register_object_symbol` logic handles
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

        // 2. Récupération du nom (StringID) de l'objet
        let symbol_node_id = typed_symbol_node.try_child(0)?;
        let symbol_node = tree.try_node(symbol_node_id)?;
        let symbol_name_id = symbol_node.try_ident()?;

        // 3. Ajout au LIR avec son nom
        ir.add_object_def(typed_object);
    }

    Ok(())
}
