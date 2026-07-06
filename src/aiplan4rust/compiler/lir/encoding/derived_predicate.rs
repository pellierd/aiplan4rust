//! Derived Predicate Encoding
//!
//! This module handles the encoding of PDDL derived predicates (axioms).
//! Derived predicates allow the domain to define new relations based on
//! existing ones, which are automatically updated as the state changes.

use crate::aiplan4rust::compiler::lir::encoding::{
    expr, typed_list, EncodingError, EncodingRegistry,
};
use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
use crate::aiplan4rust::compiler::lir::problem::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;

/// Encodes a derived predicate (axiom) from the syntax tree into the LIR.
///
/// This function translates a PDDL `:derived` definition by encoding its "head"
/// (the predicate signature and its parameters) and its "body" (the logical
/// condition that defines the predicate).
///
/// # Arguments
/// * `subtree` - The syntax subtree representing the `:derived` definition node.
/// * `registry` - The encoding registry used to manage symbol resolution and variable indices.
/// * `ir` - The mutable `LiftedProblem` where the resulting definition is registered.
///
/// # Returns
/// * `Ok(())` - If the derived predicate was successfully encoded and added to the IR.
/// * `Err(LirError)` - If the head signature is malformed or the body logic fails to encode.
///
/// # Errors
/// This function returns an error if:
/// * The predicate head or its parameters cannot be resolved.
/// * The body expression contains identifiers that are out of scope or invalid.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    builder: &mut ExprBuilder,
) -> Result<(), EncodingError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // --- STEP 1: Registry Cleanup ---
    // Clear previous variables to ensure axiom parameters (?x, ?y) start at index 0.
    registry.clear_variables();

    // --- STEP 2: Predicate Identity Resolution ---
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;
    let predicate_symbol_node_id = head_node.try_child(0)?;

    let symbol_table = registry.symbol_table();
    let declaration = symbol_table.try_get_declaration(predicate_symbol_node_id)?;

    // Retrieve the link between this axiom and the base predicate signature.
    let base_predicate_node_id = declaration.derived_source().unwrap();
    let predicate_id = registry.try_resolve_predicate(base_predicate_node_id)?;
    let head_skeleton_id = registry.try_resolve_atom_skeleton(base_predicate_node_id)?;

    // --- STEP 3: Head Encoding (Local Axiom Variables) ---
    let params_node_id = head_node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;

    // Register local variables (e.g., ?x, ?y) without clearing the registry.
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry,
        builder.store_mut(),
    )?;

    let variable_symbols = registry.get_variable_symbols();
    let head_skeleton = AtomicFormulaSkeleton::new(predicate_id, parameters)
        .with_variable_symbols(variable_symbols);

    // --- STEP 4: Body Encoding ---
    // Since the registry now contains exactly the head variables, the body
    // encoding will correctly map variable references to indices (0, 1, ...).
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;
    let body = expr::encode(
        &SyntaxSubtree::new(body_node, body_node_id, ast),
        registry,
        builder,
    )?;

    // --- STEP 5: Finalization ---
    let variable_symbols = registry.get_variable_symbols();
    let derived_predicate = DerivedPredicate::new(head_skeleton_id, head_skeleton, body)
        .with_variable_symbols(variable_symbols);

    // Register the fully encoded definition into the Lifted Problem IR.
    ir.add_derived_predicate_def(derived_predicate);
    Ok(())
}
