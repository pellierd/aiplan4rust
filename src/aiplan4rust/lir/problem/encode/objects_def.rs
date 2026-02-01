//! PDDL Object Definitions Encoding
//!
//! This module provides the core logic for encoding global symbols into the LIR.
//! It is used directly for parsing the `:objects` section of a PDDL problem,
//! and is called by the `constants_def` module to parse domain constants.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::ObjectID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{typed_symbol, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode, SyntaxSubtree, SyntaxTree};

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
pub fn encode(
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
        let object = typed_symbol::encode(&child_subtree, registry)?;
        let object_node_id = child_node.children()[0];

        // Add to LIR and retrieve the definitive ObjectID
        let object_id = ir.add_object(object);

        // Bind the name to the ID in the registry for semantic lookups
        registry.register_object(object_node_id, object_id);
    }

    Ok(())
}
