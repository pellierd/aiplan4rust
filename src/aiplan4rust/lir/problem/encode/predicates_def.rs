//! Predicate Signature Encoding
//!
//! This module handles the extraction of predicate signatures from the domain
//! AST and registers them within the LIR. It also maintains the mapping
//! between AST nodes and their corresponding LIR indices.

use std::collections::HashMap;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{atomic_formula_skeleton, named_typed_list};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes predicate definitions into the Lifted Intermediate Representation.
///
/// This function iterates through predicate declarations, converts them into
/// LIR skeletons, and populates the lookup table used for symbol resolution
/// in subsequent encoding passes.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree containing the `PredicatesDef` node.
/// * `ir` - The mutable lifted problem where predicates are registered.
/// * `ast_decl_to_lir_index` - A map to be populated with the mapping from AST `NodeId` to LIR predicate index.
///
/// # Returns
///
/// * `Ok(())` - If all predicates were successfully registered and mapped.
/// * `Err(LirError)` - If a predicate definition is malformed.
///
/// # Errors
///
/// Returns an error if an `AtomicFormulaSkeleton` cannot be constructed from
/// the provided AST node.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ir: &mut LiftedProblem,
    ast_decl_to_lir_index: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    for &child_id in subtree.node().children() {
        let child_node = tree.try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, tree);

        let predicate_id = child_node.children()[0];

        // 1. Encode the predicate signature using the free function
        let formula_skeleton = atomic_formula_skeleton::encode(&child_subtree)?;

        println!("LIR Predicate: {} -> Index {}", formula_skeleton.symbol(), ir.predicates().len());

        // 2. Register the skeleton in the IR
        ir.add_predicate(formula_skeleton);

        // 3. Map the AST NodeId to the LIR index (O(1) lookup)
        let lir_index = ir.predicates().len() - 1;
        ast_decl_to_lir_index.insert(predicate_id, lir_index);


    }

    Ok(())
}
