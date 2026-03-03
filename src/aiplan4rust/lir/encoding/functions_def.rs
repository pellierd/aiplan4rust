//! Function Signature Encoding
//!
//! This module handles the extraction of numeric function signatures (fluents)
//! from the domain AST and registers them within the LIR.
//!
//! It ensures a dual mapping in the evaluator:
//! 1. **Functor Identity**: The function's name node is mapped to a [`StringID`] (Functor).
//! 2. **Structural Signature**: The same node is mapped to a [`FunctionSkeletonID`].
//!
//! This precise binding allows the expression encoder to resolve function calls
//! during the second encoding pass by looking up the declaration symbol's IDs
//! to validate both the fluent's identity and its expected arguments.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{atomic_function_skeleton, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes function definitions into the Lifted Intermediate Representation (LIR).
///
/// This function iterates through each child of the `:functions` node (e.g., `(total-cost) - number`).
/// It performs a triple operation for each declaration:
/// 1. **Storage**: Adds the complete signature to the [`LiftedProblem`].
/// 2. **ID Retrieval**: Obtains the unique [`StringID`] (functor identity) and [`FunctionSkeletonID`].
/// 3. **Registration**: Binds the AST `NodeId` of the functor symbol to these LIR IDs.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `FunctionsDef` node.
/// * `evaluator` - The mutable evaluator for node-to-ID mapping.
/// * `ir` - The mutable Lifted Problem storage.
///
/// # Returns
///
/// * `Ok(())` - If all functions were encoded and their functors bound to LIR IDs.
/// * `Err(LirError)` - If a definition is malformed or types are unresolved.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem, // Changé de LiftedProblem à Problem selon ton code précédent
) -> Result<(), LirError> {
    let tree = subtree.tree();

    // On itère sur chaque définition de fonction (ex: (distance ?a ?b) - number)
    for &function_skeleton_node_id in subtree.node().children() {
        let function_skeleton_node = tree.try_node(function_skeleton_node_id)?;

        // 1. Extraction du nom (StringID) pour l'identité
        // Le nom est le premier enfant du nœud de la fonction
        let functor_node_id = function_skeleton_node.try_child(0)?;
        let functor_str_id = tree.try_node(functor_node_id)?.try_ident()?;

        // 2. IDENTITÉ : On réserve le FunctorID dans le Problem
        let functor_id = ir.add_function_symbol(functor_str_id);

        // 3. ENCODE : On construit le squelette structurel
        // On lui passe l'ID pour qu'il n'ait pas à manipuler de StringID
        let function_skeleton_subtree = SyntaxSubtree::new(
            function_skeleton_node,
            function_skeleton_node_id,
            tree
        );
        let function_skeleton = atomic_function_skeleton::encode(
            &function_skeleton_subtree,
            registry,
            functor_id // Passé ici
        )?;

        // 4. STOCKAGE : On enregistre la définition complète
        // Utilise ta version mise à jour de add_function_def (qui ne prend plus de StringID)
        let function_skeleton_id = ir.add_function_def(function_skeleton);

        // 5. MAPPING : On lie le NodeId de l'AST aux IDs du LIR
        registry.register_function_skeleton(functor_node_id, function_skeleton_id);
        registry.register_functor(functor_node_id, functor_id);
    }

    Ok(())
}
