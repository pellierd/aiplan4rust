//! PDDL Object Definitions Encoding
//!
//! This module provides the core ops for encoding global symbols into the LIR.
//! It is used directly for parsing the `:objects` section of a PDDL problem,
//! and is called by the `constants_def` module to parse domain constants.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::encoding::{typed_symbol, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::semantic::symbol::origin::Origin;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encodes a list of objects from the syntax tree into the Lifted Intermediate Representation (LIR).
///
/// This function coordinates the encoding process in two distinct phases to ensure
/// consistency and optimal performance:
///
/// 1. **Phase 1 (Collection & Filtering)**: Identifies valid object nodes, filters out
///    `Shared` domain constants using the `SymbolTable`, and registers their `SymbolId`
///    in the `EncodingRegistry`.
/// 2. **Phase 2 (Definition & Typing)**: Processes the validated nodes to resolve their
///    types and stores the final `TypedSymbol` definitions in the `LiftedProblem`.
///
/// This separation ensures that shared constants are only encoded once (in the domain)
/// and prevents redundant lookups in the symbol table.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the typed list (e.g., from `:objects`).
/// * `registry` - The mutable encoding registry used for symbol-to-ID binding and lookups.
/// * `ir` - The mutable Lifted Problem where the filtered objects are stored.
///
/// # Returns
///
/// * `Ok(())` - If all objects were successfully filtered, registered, and defined.
/// * `Err(LirError)` - If a symbol is missing from the table or typing resolution fails.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // Phase 1: Register all object names to generate their ObjectIDs in the registry
    let object_nodes_ids = collect_object_ids(subtree, registry, ir)?;

    // Phase 2: Resolve types and finalize the object definitions in the LIR
    encode_definitions(subtree, registry, ir, object_nodes_ids)?;

    Ok(())
}

/// Phase 1: Collects object identifiers, filters them, and prepares the encoding whitelist.
///
/// This function performs three critical tasks:
/// 1. **Filtering**: It skips objects marked as `Origin::Shared` (domain constants) to
///    ensure the LIR only contains problem-specific objects.
/// 2. **Registration**: It registers valid `SymbolId`s in the `EncodingRegistry` and
///    adds them to the `LiftedProblem` (IR).
/// 3. **Whitelist Generation**: It returns a `Vec<NodeId>` containing only the
///    `TypedSymbol` nodes that passed the filter, to be used by Phase 2.
///
/// This dual-phase approach prevents redundant `SymbolTable` lookups and ensures
/// consistency between object registration and definition encoding.
///
/// # Returns
/// A `Result` containing a `Vec<NodeId>` of validated object nodes to be defined in Phase 2.
fn collect_object_ids(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<Vec<NodeId>, LirError> {
    let tree = subtree.tree();
    let list_node = tree.try_node(subtree.node().try_child(0)?)?;
    let mut object_nodes_ids = Vec::new();

    for typed_symbol_id in list_node.children() {
        let typed_symbol_node = tree.try_node(*typed_symbol_id)?;

        // In PDDL AST, the first child of a TypedSymbol node is the identifier (name)
        let symbol_node_id = typed_symbol_node.try_child(0)?;
        let symbol_node = tree.try_node(symbol_node_id)?;
        let symbol_id = symbol_node.try_ident()?;

        // --- SHARED CONSTANT FILTER ---
        // We skip objects marked as 'Shared' by the Linker.
        // These domain constants are already encoded globally in the LiftedDomain.
        let decl = registry
            .symbol_table()
            .try_get_declaration_from(symbol_id, symbol_node_id)?;

        if decl.origin() == Origin::Shared {
            continue;
        }

        // Register the object. Your `register_object_symbol` handles
        // the NodeId mapping and ID generation.
        registry.register_object_symbol(symbol_id, symbol_node_id);
        ir.add_object_symbol(symbol_id);

        // Add to the whitelist for Phase 2
        object_nodes_ids.push(*typed_symbol_id);
    }

    Ok(object_nodes_ids)
}

/// Phase 2: Resolves object types and adds full definitions to the LIR.
///
/// This function processes only the nodes provided in `object_nodes_ids`.
/// These nodes must have been pre-filtered (e.g., to exclude shared domain
/// constants) and registered in the `EncodingRegistry` during Phase 1.
///
/// For each valid node, it:
/// 1. Creates a local subtree for the typed symbol.
/// 2. Resolves the `TypeID` using the `typed_symbol` module.
/// 3. Adds the resulting `TypedSymbol<ObjectID, TypeID>` to the `LiftedProblem`.
///
/// # Arguments
/// * `subtree` - The context of the original AST subtree.
/// * `registry` - The encoding registry used for type and symbol resolution.
/// * `ir` - The Lifted Problem being constructed.
/// * `object_nodes_ids` - The list of pre-validated AST node IDs to encode.
fn encode_definitions(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    object_nodes_ids: Vec<NodeId>,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    for typed_symbol_id in object_nodes_ids {
        let typed_symbol_node = tree.try_node(typed_symbol_id)?;
        let child_subtree = SyntaxSubtree::new(typed_symbol_node, typed_symbol_id, tree);

        // 1. Encoding structure (ID + Type)
        // Note: Types are resolved here based on the pre-validated symbol nodes.
        let typed_object = typed_symbol::encode_typed_object(&child_subtree, registry)?;

        // 2. Add definition to the LIR
        ir.add_object_def(typed_object)?;
    }

    Ok(())
}
