use std::collections::{HashMap, HashSet};
use logos::Source;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{Type, TypeID, TypedSymbol};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{typed_symbol, EncodingContext};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxSubtree};

/// Encodes type definitions from the AST into the Lifted Intermediate Representation (LIR).
///
/// This function iterates through the type declarations in the AST, registers them
/// within the `LiftedProblem`, and maintains a mapping between AST node IDs and
/// their corresponding internal `TypeID`.
///
/// # Technical Note: Temporal Hack
/// The current implementation uses `ir.types().count() - 1` to retrieve the latest `TypeID`.
/// This is a **temporal hack** because the underlying storage is currently a `HashMap`
/// (which is unordered and inefficient to count repeatedly).
/// Once the IR refactor to a `Vector`-based storage is complete, this should be
/// replaced by a direct index return from `ir.add_type()`.
///
/// # Arguments
/// * `subtree` - The syntax subtree pointing to the `TypesDef` node.
/// * `ir` - The mutable reference to the Lifted Problem being constructed.
/// * `ast_type_to_id` - A map to be populated with `NodeId -> TypeID` associations.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        // 1. Encodage sémantique : on récupère la déclaration (nom + parents)
        let type_declaration = typed_symbol::encode(&child_subtree)?;

        // On crée la structure sémantique qui servira de clé
        let ty = Symbol::new(type_declaration.symbol(), SymbolKind::PrimitiveType);

        // 2. Ajout au LIR pour obtenir l'ID unique
        ir.add_type(type_declaration);
        let type_id = TypeID::new(ir.types().count() - 1);

        // 3. Enregistrement dans la table du Registre
        // Maintenant, 'ty' (la structure) pointe vers 'type_id'
        registry.register_type(ty, type_id);
    }

    Ok(())
}
