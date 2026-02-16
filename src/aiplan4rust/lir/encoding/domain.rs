//! Domain Encoding Module
//!
//! This module provides the infrastructure to transform a PDDL domain AST into
//! a Lifted Intermediate Representation (LIR).
//!
//! The transformation is designed as a two-pass compiler:
//! 1. **Structural Collection**: Registering names and signatures to build a global symbol map.
//! 2. **Logic Binding**: Encoding expr and actions by resolving symbols
//!    against the maps created in the first pass.
//!
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::encoding::{action, predicates_def, functions_def, types_def, constants_def, expr, durative_action, method, derived_predicate, task};
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{Node, SyntaxSubtree, Tree};

/// Encodes the PDDL domain into the Lifted Intermediate Representation (LIR).
///
/// This process is performed in two distinct normalization:
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
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    // 1. Collection Phase: Fill the IR skeletons and the mapping tables.
    // We do NOT return the EncodingContext here to avoid borrow checker conflicts
    // between the mutable maps and the context itself.
    collect_definitions(syntax_tree, registry, ir)?;

    // 3. Logic Encoding Phase:
    // Since 'ctx' only borrows the maps and NOT the 'ir', Rust allows
    // passing 'ir' as a mutable reference here.
    encode_logic(syntax_tree, registry, ir)?;

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
/// registered and available for resolution in expr.
///
/// # Arguments
///
/// * `context` - The semantic context providing access to the domain syntax tree.
/// * `ir` - The mutable LiftedProblem where skeletons and definitions are stored.
/// * `ast_pred_to_id` - A mutable map to be populated with predicate mappings.
/// * `ast_func_to_id` - A mutable map to be populated with function mappings.
///
/// # Errors
///
/// Returns a `LirError` if any structural definition is malformed or if mandatory
/// identifiers are missing.
fn collect_definitions(
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,

) -> Result<(), LirError> {

    for (node_id, node) in syntax_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, syntax_tree);

        match node.kind() {
            AstKind::DomainName => ir.set_domain_name(node.try_ident()?)?,
            AstKind::TypesDef => types_def::encode(&subtree, registry, ir)?,
            AstKind::ConstantsDef => constants_def::encode(&subtree, registry, ir)?,
            AstKind::PredicatesDef => predicates_def::encode(&subtree, registry, ir)?,
            AstKind::FunctionsDef => functions_def::encode(&subtree, registry, ir)?,
            AstKind::TaskDef => task::encode(&subtree, registry, ir)?,
            AstKind::ActionDef => task::encode(&subtree, registry, ir)?,
            AstKind::DurativeActionDef => task::encode(&subtree, registry, ir)?,
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
/// * `Err(LirError)` - If an expression fails to encoding or if a symbol remains unresolved.
///
/// # Note
///
/// Elements already processed in the collection phase (like Types or Constants)
/// are ignored in this pass.
fn encode_logic(
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    for (node_id, node) in syntax_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, syntax_tree);

        match node.kind() {
            AstKind::Constraints => {
                let constraints = expr::encode(&subtree, registry)?;
                ir.set_domain_constraints(constraints);
            }
            AstKind::ActionDef => action::encode(&subtree, registry, ir)?,
            AstKind::DurativeActionDef => {
                let action = durative_action::encode(&subtree, registry)?;
                ir.add_durative_action_def(action);
            }
            AstKind::DerivedDef => {
                let derived_predicate = derived_predicate::encode(&subtree, registry)?;
                ir.add_derived_predicate_def(derived_predicate);
            }
            AstKind::MethodDef => {
                let method = method::encode(&subtree, registry)?;
                ir.add_method_def(method);
            }
            _ => {} 
        }
    }

    Ok(())
}
