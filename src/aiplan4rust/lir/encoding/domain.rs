//! Domain Encoding Module
//!
//! This module provides the infrastructure to transform a PDDL domain AST into
//! a Lifted Intermediate Representation (LIR).
//!
//! The transformation is designed as a two-pass compiler:
//! 1. **Structural Collection**: Registering names and signatures to build a global symbol map.
//! 2. **Logic Binding**: Encoding logic and actions by resolving symbols
//!    against the maps created in the first pass.
//!

use crate::aiplan4rust::lir::encoding::{
    action, constants_def, constraints, derived_predicate, functions_def, method, predicates_def,
    preference, task, types_def, EncodingError, EncodingRegistry,
};
use crate::aiplan4rust::lir::expr::ExprBuilder;
use crate::aiplan4rust::lir::problem::skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::{Requirement, SymbolId, Type, TypeId, TypedList};
use crate::aiplan4rust::syntax::ast::tree::{Node, NodeId, SyntaxSubtree, Tree};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};

/// Encodes the PDDL domain into the Lifted Intermediate Representation (LIR).
///
/// This process is performed in two distinct logic:
/// 1. **Collection Pass**: Populates the IR with structural definitions (types, constants,
///    and signatures for predicates/functions) and maps their AST NodeIds to LIR indices.
/// 2. **Logic Pass**: Encodes complex business ops (action bodies, durative actions,
///    and constraints) using the resolved symbol context from the first pass.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the domain AST and symbol table.
/// * `ir` - The mutable LiftedProblem to be populated.
/// * `ast_pred_to_idx` - A mutable map to old the mapping between predicate AST nodes and their LIR indices.
/// * `ast_func_to_idx` - A mutable map to old the mapping between function AST nodes and their LIR indices.
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
    builder: &mut ExprBuilder, // Injection indispensable du builder
) -> Result<(), EncodingError> {
    // 1. Collection Phase: Populate IR skeletons and mapping tables.
    // This is separated to avoid simultaneous mutable borrows of the IR
    // while traversing the symbol tables.
    collect_definitions(syntax_tree, registry, ir)?;

    // 2. Built-in Phase: Ensure support PDDL functions are registered.
    encode_builtin_functions(registry, ir)?;

    // 3. Logic Encoding Phase:
    // At this stage, the symbol registry is populated, allowing 'encode_logic'
    // to resolve atom parameters and fluents while modifying the 'ir'.
    encode_logic(syntax_tree, registry, ir, builder)?;

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
/// registered and available for resolution in logic.
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
) -> Result<(), EncodingError> {
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
            AstKind::Preference => preference::encode(&subtree, registry, ir)?,

            _ => {}
        }
    }
    Ok(())
}

/// Performs the second pass of the domain encoding by processing the behavioral ops.
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
/// * `ir` - The mutable LiftedProblem where the encoded ops is stored.
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
    builder: &mut ExprBuilder, // Injection indispensable du builder
) -> Result<(), EncodingError> {
    for (node_id, node) in syntax_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, syntax_tree);

        match node.kind() {
            AstKind::Constraints => {
                let constraints = constraints::encode(&subtree, registry, builder)?;
                ir.set_domain_constraints(constraints);
            }
            AstKind::ActionDef | AstKind::DurativeActionDef => {
                action::encode(&subtree, registry, ir, builder)?
            }
            AstKind::DerivedDef => derived_predicate::encode(&subtree, registry, ir, builder)?,
            AstKind::MethodDef => method::encode(&subtree, registry, ir, builder)?,
            _ => {}
        }
    }

    Ok(())
}

/// Injects predefined system functions (built-ins) into the encoding registry.
///
/// This step is mandatory for supporting special PDDL functions such as `total-cost`
/// (required by the `:action-costs` requirement) and `total-time` (used in temporal
/// or numeric domains).
///
/// # Architecture and LIR Consistency
///
/// Unlike user-defined functions found in the `(:functions ...)` block, these
/// system functions do not have a physical representation in the domain's AST.
/// To maintain architectural integrity:
///
/// 1. They are mapped to reserved **virtual** [`NodeId`]s (defined in [`EncodingRegistry`]).
/// 2. They are registered exclusively in the [`EncodingRegistry`] to allow symbol
///    resolution during expression encoding.
/// 3. They are **not** added to the problem definitions ([`ir.function_defs`]). This
///    prevents indexing mismatches between the LIR vectors and the AST nodes.
///
/// # Supported Requirements
///
/// * [`Requirement::ActionCosts`]: Registers the `total-cost` function.
/// * [`Requirement::Fluents`] | [`Requirement::NumericFluents`]: Registers the `total-time` function.
///
/// # Errors
///
/// Returns a [`LirError`] if the registration of a system function fails within the
/// registry (e.g., due to an unexpected internal symbol collision).
///
/// # Arguments
///
/// * `registry` - The encoding registry where system function signatures are injected.
/// * `ir` - The lifted problem, used as a read-only reference to check active requirements.
pub fn encode_builtin_functions(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    let reqs = ir.requirements();

    let has_action_costs = reqs.contains(&Requirement::ActionCosts);
    let has_numeric_fluents =
        reqs.contains(&Requirement::Fluents) || reqs.contains(&Requirement::NumericFluents);
    let has_durative = reqs.contains(&Requirement::DurativeActions);

    // Case: Action Costs -> Register 'total-cost'
    if has_action_costs {
        register_builtin_function(
            registry,
            ir,
            SymbolInterner::TOTAL_COST_SYMBOL_ID,
            EncodingRegistry::TOTAL_COST_NODE_ID,
            TypeId::NUMBER_TYPE_ID,
        )?;
    }

    // Case: Fluents/Numeric OR Durative Actions -> Register 'total-time'
    // Note: Using || (logical OR) instead of | (bitwise OR) for clarity and short-circuiting
    if has_numeric_fluents || has_durative {
        register_builtin_function(
            registry,
            ir,
            SymbolInterner::TOTAL_TIME_SYMBOL_ID,
            EncodingRegistry::TOTAL_TIME_NODE_ID,
            TypeId::NUMBER_TYPE_ID,
        )?;
    }

    Ok(())
}

/// Injects a system-defined function into both the LIR and the encoding registry.
///
/// This helper synchronizes the creation of a built-in function by:
/// 1. Inserting the [`FunctionSymbol`] into the [`LiftedProblem`].
/// 2. Defining its [`AtomicFunctionSkeleton`] (arity 0) with the specified return type.
/// 3. Mapping both to a reserved virtual [`NodeId`] within the [`EncodingRegistry`].
///
/// This dual registration allows the encoder to resolve implicit PDDL symbols
/// (like `total-time`) as if they were standard declared functions.
///
/// # Arguments
///
/// * `registry` - The active [`EncodingRegistry`] to be updated with virtual bindings.
/// * `ir` - The [`LiftedProblem`] where the symbol and definition are stored.
/// * `symbol_id` - The unique identifier from the interner (e.g., `TOTAL_TIME_SYMBOL_ID`).
/// * `virtual_node_id` - The reserved [`NodeId`] used as a stable key for resolution.
/// * `return_type` - The [`TypeId`] of the value returned by this function (typically numeric).
///
/// # Errors
///
/// Returns a [`LirError`] if the registration process fails or if there is a
/// conflict in the registry.
fn register_builtin_function(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    symbol_id: SymbolId,
    virtual_node_id: NodeId,
    return_type: TypeId,
) -> Result<(), EncodingError> {
    // 1. Register the function symbol and its virtual mapping
    let sym = ir.add_function_symbol(symbol_id);
    registry.register_functor(virtual_node_id, sym);

    // 2. Define and register the function skeleton (signature)
    // System functions like total-time/total-cost always have an empty parameter list.
    let skeleton =
        AtomicFunctionSkeleton::new(sym, TypedList::empty(), Type::primitive(return_type));

    let def_id = ir.add_function_def(skeleton);
    registry.register_function_skeleton(virtual_node_id, def_id);

    Ok(())
}
