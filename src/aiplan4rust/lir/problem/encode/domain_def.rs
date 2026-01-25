//! Domain Encoding Module
//!
//! This module provides the infrastructure to transform a PDDL domain AST into
//! a Lifted Intermediate Representation (LIR).
//!
//! The transformation is designed as a two-pass compiler:
//! 1. **Structural Collection**: Registering names and signatures to build a global symbol map.
//! 2. **Logic Binding**: Encoding expressions and actions by resolving symbols
//!    against the maps created in the first pass.
//!
use std::collections::HashMap;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::encode::{action_def, predicates_def, functions_def, types_def, constants_def, expr, durative_action_def, method_def, derived_predicate_def, task_def};
use crate::aiplan4rust::lir::problem::encode::context::EncodingContext;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};

/// Encodes the PDDL domain into the Lifted Intermediate Representation (LIR).
///
/// This process is performed in two distinct passes:
/// 1. **Collection Pass**: Populates the IR with structural definitions (types, constants,
///    and signatures for predicates/functions) and maps their AST NodeIds to LIR indices.
/// 2. **Logic Pass**: Encodes complex business logic (action bodies, durative actions,
///    and constraints) using the resolved symbol context from the first pass.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the domain AST and symbol table.
/// * `ir` - The mutable LiftedProblem to be populated.
/// * `ast_pred_to_idx` - A mutable map to store the mapping between predicate AST nodes and their LIR indices.
/// * `ast_func_to_idx` - A mutable map to store the mapping between function AST nodes and their LIR indices.
///
/// # Returns
///
/// * `Ok(())` - If the domain was successfully encoded.
/// * `Err(LirError)` - If an error occurred during encoding (e.g., syntax mismatch,
///   duplicate definitions, or failed symbol resolution).
///
/// # Errors
///
/// This function will return an error if:
/// * The AST structure is inconsistent with the expected PDDL format.
/// * Identifiers used in actions or constraints cannot be found in the domain's symbol table.
pub(crate) fn encode(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
    ast_pred_to_idx: &mut HashMap<NodeId, usize>,
    ast_func_to_idx: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {

    // 1. Collection Phase: Fill the IR skeletons and the mapping tables.
    // We do NOT return the EncodingContext here to avoid borrow checker conflicts
    // between the mutable maps and the context itself.
    collect_definitions(context, ir, ast_pred_to_idx, ast_func_to_idx)?;

    // 2. Context Creation: The context borrows the tables declared in the caller's scope.
    let symbol_table = context.domain_table();
    let ctx = EncodingContext::new(symbol_table, &ast_pred_to_idx, &ast_func_to_idx);

    // 3. Logic Encoding Phase:
    // Since 'ctx' only borrows the maps and NOT the 'ir', Rust allows
    // passing 'ir' as a mutable reference here.
    encode_logic(context, &ctx, ir)?;

    Ok(())
}

/// Performs the first pass of the domain encoding by collecting all structural definitions.
///
/// This function traverses the domain's Abstract Syntax Tree (AST) to extract:
/// - Basic metadata (Domain name).
/// - Type hierarchies and global constants.
/// - Predicate and function signatures, mapping their AST IDs to unique LIR indices.
/// - Task skeletons for HTN (Hierarchical Task Network) planning.
///
/// This phase must be completed before `encode_logic` to ensure all symbols are
/// registered and available for resolution in expressions.
///
/// # Arguments
///
/// * `context` - The semantic context providing access to the domain syntax tree.
/// * `ir` - The mutable LiftedProblem where skeletons and definitions are stored.
/// * `ast_pred_to_idx` - A mutable map to be populated with predicate mappings.
/// * `ast_func_to_idx` - A mutable map to be populated with function mappings.
///
/// # Errors
///
/// Returns a `LirError` if any structural definition is malformed or if mandatory
/// identifiers are missing.
fn collect_definitions(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
    ast_pred_to_idx: &mut HashMap<NodeId, usize>,
    ast_func_to_idx: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    let domain_tree = context.domain_syntax_tree();

    for (node_id, node) in domain_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, domain_tree);

        match node.kind() {
            AstKind::DomainName => ir.set_domain_id(node.try_ident()?)?,
            AstKind::TypesDef => ir.add_types(types_def::endode(&subtree)?),
            AstKind::ConstantsDef => ir.add_constants(constants_def::encode(&subtree)?),
            AstKind::PredicatesDef => {
                predicates_def::encode(&subtree, ir, ast_pred_to_idx)?;
            }
            AstKind::FunctionsDef => {
                functions_def::encode(&subtree, ir, ast_func_to_idx)?;
            }
            AstKind::TaskDef => {
                let task = task_def::encode(&subtree)?;
                ir.add_task(task);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Performs the second pass of the domain encoding by processing the behavioral logic.
///
/// This function relies on the `EncodingContext` populated during the first pass
/// (`collect_definitions`) to resolve predicate and function identifiers into their
/// corresponding LIR indices.
///
/// It handles the encoding of:
/// - Global domain constraints.
/// - Action bodies (preconditions and effects).
/// - Durative actions, derived predicates, and HTN methods.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the domain AST.
/// * `ctx` - The encoding context used to resolve symbols (predicates, functions, etc.).
/// * `ir` - The mutable LiftedProblem where the encoded logic is stored.
///
/// # Returns
///
/// * `Ok(())` - If all logical elements were successfully encoded and bound.
/// * `Err(LirError)` - If an expression fails to encode or if a symbol remains unresolved.
///
/// # Note
///
/// Elements already processed in the collection phase (like Types or Constants)
/// are ignored in this pass.
fn encode_logic(
    context: &LinkedSemanticContext,
    ctx: &EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let domain_tree = context.domain_syntax_tree();

    for (node_id, node) in domain_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, domain_tree);

        match node.kind() {
            AstKind::Constraints => {
                let constraints = expr::encode(&subtree, ctx, ir)?;
                ir.set_domain_constraints(constraints);
            }
            AstKind::ActionDef => {
                let action = action_def::encode(&subtree, ctx, ir)?;
                ir.add_action(action);
            }
            AstKind::DurativeActionDef => {
                let action = durative_action_def::encode(&subtree, &ctx, ir)?;
                ir.add_durative_action(action);
            }
            AstKind::DerivedDef => {
                let derived_predicate = derived_predicate_def::encode(&subtree, &ctx, ir)?;
                ir.add_derived_predicate(derived_predicate);
            }
            AstKind::MethodDef => {
                let method = method_def::encode(&subtree, &ctx, ir)?;
                ir.add_method(method);
            }
            _ => {} 
        }
    }

    Ok(())
}
