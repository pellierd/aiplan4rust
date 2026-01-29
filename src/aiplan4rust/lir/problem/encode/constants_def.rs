//! Global Symbol Encoding
//!
//! This module handles the extraction of constants (from domain) and objects
//! (from problem). It ensures that all symbols are parsed with their
//! respective types and stored in a collection for global resolution.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::ObjectID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{typed_symbol, EncodingContext};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes a collection of constants or objects from a syntax subtree.
///
/// This function parses a list of typed symbols (e.g., `:constants` or `:objects` blocks)
/// and returns them as a `HashSet`. This ensures that each symbol name is unique
/// within the global scope.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the constant or object definition block.
///
/// # Returns
///
/// * `Ok(HashSet<TypedSymbol>)` - A set of all uniquely identified typed symbols.
/// * `Err(LirError)` - If the list structure is malformed or a symbol cannot be parsed.
///
/// # Errors
///
/// This function returns an error if:
/// * The child node containing the actual list of symbols cannot be accessed.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    context: &mut EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // 1. Accès au premier enfant (le conteneur de la liste d'objets)
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    // 2. Transformation directe des enfants en TypedSymbol
    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        let object = typed_symbol::encode(&child_subtree)?;
        let object_symbol = Symbol::new(object.symbol(), SymbolKind::Constant);

        ir.add_constant(object);
        let object_id = ObjectID::new(ir.constants().count() - 1);
        context.register_object(object_symbol, object_id);
    }

    Ok(())
}
/*pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    context: &mut EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // 1. Retrieve the first child (the container of the typed list)
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    // 2. Iterate over children to transform each element into a TypedSymbol
    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        // Encode the syntax into a TypedSymbol declaration
        let type_declaration = typed_symbol::encode(&child_subtree)?;
        let ty = type_declaration.ty().clone();

        // Register the type in the IR
        ir.add_type(type_declaration);

        // HACK: Calculate the ID based on current count.
        // Inefficient (O(n^2) total) but works until the IR uses a Vec.
        let type_id = TypeID::new(ir.types().count() - 1);

        // Map the AST NodeId to the newly created TypeID for future binding
        context.register_type(ty, type_id);
    }

    Ok(())
}*/
