//! PDDL Type Definition Encoding
//!
//! This module implements the two-pass encoding process for PDDL types:
//! 1. **Phase 1 (Discovery):** Scans all type names to populate the registry with unique `TypeID`s.
//! 2. **Phase 2 (Definition):** Resolves inheritance relationships and adds full type declarations to the LIR.

use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{typed_symbol, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{Node, SyntaxSubtree};

/// Encodes the PDDL `:types` section into the Lifted Intermediate Representation (LIR).
///
/// This function coordinates a two-pass process to ensure that types can reference 
/// each other regardless of their declaration order in the AST.
///
/// # Arguments
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `registry` - The encoding context used to map symbols to unique `TypeID`s.
/// * `ir` - The Lifted Problem where the final type declarations are stored.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    // Phase 1: Register all type symbols to generate their TypeIDs
    collect_type_ids(subtree, registry)?;

    // Phase 2: Encode the semantic definitions (inheritance and properties)
    encode_definitions(subtree, registry, ir)?;

    Ok(())
}

/// Phase 1: Collects all type identifiers and assigns them unique IDs in the registry.
///
/// This pass performs an initial scan of the `:types` block to register every type name
/// before any inheritance resolution occurs. This ensures that Phase 2 can safely
/// resolve parent-child relationships regardless of the order in which types appear
/// in the PDDL file.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `registry` - The mutable encoding context where type symbols are mapped to new `TypeID`s.
///
/// # Returns
///
/// * `Ok(())` - If all type identifiers were successfully discovered and registered.
/// * `Err(LirError)` - If the AST structure is unexpected or identifiers cannot be extracted.
///
/// # Errors
///
/// This function will return an error if:
/// * The expected list of typed symbols is missing from the AST.
/// * A node that should contain a type name fails to provide a valid identifier via `try_ident()`.
fn collect_type_ids(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry
) -> Result<(), LirError> {
    let tree = subtree.tree();
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = tree.try_node(list_node_id)?;

    for typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(*typed_symbol_id)?;
        let ty_node_id = typed_symbol_node.children()[0];
        registry.register_type_symbol(ty_node_id);
    }
    Ok(())
}

/// Phase 2: Resolves inheritance and adds full type declarations to the LIR.
///
/// This pass iterates through the same type list as Phase 1, but focuses on
/// semantic resolution. It uses the `typed_symbol` module to parse the
/// relationship between type names and their parent types, then stores the
/// final `TypeDeclaration` in the LIR.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `TypesDef` node.
/// * `registry` - The encoding context where `TypeIDs` were registered in Phase 1.
/// * `ir` - The mutable reference to the `LiftedProblem` where declarations are stored.
///
/// # Returns
///
/// * `Ok(())` - If all type definitions were successfully resolved and added to the LIR.
/// * `Err(LirError)` - If a type reference cannot be found in the registry or the AST is invalid.
///
/// # Errors
///
/// This function will return an error if:
/// * `typed_symbol::encode` fails (e.g., a parent type was not declared in Phase 1).
/// * The AST structure prevents navigating to the child nodes of the type list.
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

        // Encode the TypedSymbol which now can resolve its parent TypeIDs from the registry
        let type_declaration = typed_symbol::encode(&child_subtree, registry)?;

        // Store the final declaration in the LIR
        ir.add_type(type_declaration);
    }

    Ok(())
}
