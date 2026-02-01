//! Predicate Definitions Encoding
//!
//! This module orchestrates the extraction of predicate signatures (skeletons)
//! from the domain AST and registers them within the LIR.
//!
//! It ensures a dual mapping in the registry:
//! 1. **Logical Identity**: The predicate's name node is mapped to a [`PredicateID`].
//! 2. **Structural Signature**: The same node is mapped to an [`AtomSkeletonID`].
//!
//! This precise binding allows formulas (preconditions, effects, etc.) to resolve
//! atom occurrences back to their full LIR definition and unique identity during
//! the second pass.

use crate::aiplan4rust::lang::PredicateID;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{atomic_formula_skeleton, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes the `:predicates` section of a PDDL domain into the LIR.
///
/// This function iterates through each predicate declaration (e.g., `(at ?r - robot)`).
/// It extracts the formal signature and performs a triple operation:
/// 1. **Storage**: Adds the signature to the [`LiftedProblem`].
/// 2. **ID Retrieval**: Obtains the newly generated [`PredicateID`] and [`AtomSkeletonID`].
/// 3. **Registration**: Binds the AST `NodeId` of the predicate symbol to these LIR IDs.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `PredicatesDef` node.
/// * `registry` - The mutable registry for node-to-ID mapping.
/// * `ir` - The mutable Lifted Problem storage.
///
/// # Returns
///
/// * `Ok(())` - If all predicate signatures were successfully encoded and registered.
/// * `Err(LirError)` - If a predicate structure is invalid or type resolution fails.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let tree = subtree.tree();

    // Iterate over each predicate definition (e.g., `(at ?r - robot ?l - location)`)
    for &atom_skeleton_node_id in subtree.node().children() {
        let atom_skeleton_node = tree.try_node(atom_skeleton_node_id)?;
        let atom_skeleton_subtree = SyntaxSubtree::new(
            atom_skeleton_node,
            atom_skeleton_node_id,
            tree
        );

        // 1. Build the skeleton (Symbol + Typed Parameters)
        // This delegates to atomic_formula_skeleton, which uses named_typed_list.
        let atom_skeleton = atomic_formula_skeleton::encode(&atom_skeleton_subtree, registry)?;

        // 2. Identify the Predicate Name NodeId
        // We register the ID of the symbol itself (the "at" in "(at ?r ?l)")
        // to stay consistent with how the symbol table identifies declarations.
        let predicate_node_id = atom_skeleton_subtree.node().children()[0];

        // 3. Physical storage in the LIR
        // The LIR returns both the logical PredicateID and the structural AtomSkeletonID.
        let (predicate_id, atom_skeleton_id) = ir.add_atom_skeleton(atom_skeleton);

        // 4. Node mapping in the registry.
        // We link the declaration's NodeId to both LIR IDs. This allows the
        // expression encoder to resolve an atom call to its full context.
        registry.register_atom_skeleton(predicate_node_id, atom_skeleton_id);
        registry.register_predicate(predicate_node_id, predicate_id);
    }

    Ok(())
}
