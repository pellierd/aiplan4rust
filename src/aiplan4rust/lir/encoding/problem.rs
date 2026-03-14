//! Problem Encoding Orchestration
//!
//! This module iterates through the PDDL problem AST to extract instance-specific
//! information such as objects, the initial state, goal conditions, and HTN
//! initial task networks. It populates the final `LiftedProblem` IR.

use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Requirement, SymbolId, Type, TypeId, TypedList};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{expr, goal, init, initial_task_network, objects_def, EncodingRegistry};
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{Node, NodeId, SyntaxSubtree, Tree};

/// Reserved [`NodeId`] for the implicit `total-time` function.
/// Maps to the temporal fluent representing elapsed plan time.
pub const TOTAL_TIME_NODE_ID: NodeId = NodeId::new(usize::MAX - 1);

/// Reserved [`NodeId`] for the implicit `total-cost` function.
/// Maps to the numeric fluent representing cumulative action costs.
pub const TOTAL_COST_NODE_ID: NodeId = NodeId::new(usize::MAX - 2);

/// Encodes a PDDL/HTN problem AST into the Lifted Intermediate Representation (LIR).
///
/// This function performs a pre-order traversal of the problem's syntax tree. It
/// resolves symbols (predicates and functions) using the mapping indices provided
/// from the domain encoding phase.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the problem's AST and symbol table.
/// * `ir` - The mutable `LiftedProblem` to be populated with initial states, goals, and objects.
/// * `ast_pred_to_idx` - A lookup map to resolve AST predicate references to LIR indices.
/// * `ast_func_to_idx` - A lookup map to resolve AST function references to LIR indices.
///
/// # Returns
///
/// * `Ok(())` - If the problem was successfully encoded.
/// * `Err(LirError)` - If any semantic error (missing symbols) or syntax error is found.
///
/// # Errors
///
/// This function returns an error if:
/// * Symbol resolution fails for the initial state or goal.
/// * The problem name or object definitions are malformed.
/// * A logical expression (metric, constraint, length) fails to encoding.
/// Encodes a high-level AST into a Lifted Intermediate Representation (LIR).
///
/// the encoding process is organized into three distinct phases:
/// 1. **Built-in Initialization**: Set up system-defined symbols (e.g., `total-time`).
/// 2. **Structural Collection**: Map domain/problem declarations (types, functions, etc.).
/// 3. **Logic Encoding**: Transform expressions, initial state, and goals into LIR.
///
/// # Errors
///
/// Returns a [`LirError`] if any phase of the encoding fails (e.g., symbol resolution error).
pub fn encode(
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {

    // Phase 0: Initialize system-level built-ins and numeric requirements.
    // This handles symbols that are implicitly defined by the PDDL specification.
    initialize_builtins(registry, ir)?;

    // Phase 1: Structural Declarations.
    // Collect and register user-defined types, constants, predicates, and functions.
    collect_problem_definitions(syntax_tree, registry, ir)?;

    // Phase 2: Problem Logic.
    // Encode the initial state, goal conditions, and metric expressions.
    encode_problem_logic(syntax_tree, registry, ir)?;

    Ok(())
}


/// Initializes built-in functions and types required by the PDDL specification.
///
/// This function handles the "pre-binding" of system-defined symbols like `total-time`.
/// It registers these symbols in the `LiftedProblem` (LIR) and maps them in the
/// `EncodingRegistry` using a reserved virtual `NodeId` (`TOTAL_TIME_NODE_ID`).
///
/// This ensures that subsequent encoding passes can resolve these built-ins
/// without needing explicit declarations in the PDDL domain file.
///
/// # Errors
///
/// Returns a [`LirError`] if type registration or symbol insertion fails.
/// Initializes built-in functions and types required by the PDDL specification.
///
/// This phase ensures that implicit symbols like `total-time` and `total-cost`
/// are correctly defined in the LIR and mapped in the registry before
/// processing user-defined expressions.
pub fn initialize_builtins(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let requirements = ir.requirements();

    // Built-in numeric functions are only relevant if fluents are enabled.
    if requirements.contains(&Requirement::Fluents) || requirements.contains(&Requirement::NumericFluents) {

        // Ensure the primitive 'number' type is initialized.
        let number_type = registry.register_number_type();

        // Register 'total-time': Represents elapsed plan time.
        register_system_function(
            registry,
            ir,
            SymbolInterner::TOTAL_TIME_SYMBOL_ID,
            TOTAL_TIME_NODE_ID,
            number_type,
        )?;

        // Register 'total-cost': Represents cumulative action costs. TO DO for ACTION COST
        /*register_system_function(
            registry,
            ir,
            SymbolInterner::TOTAL_COST_SYMBOL_ID,
            TOTAL_COST_NODE_ID,
            number_type,
        )?;*/
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
fn register_system_function(
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
    symbol_id: SymbolId,
    virtual_node_id: NodeId,
    return_type: TypeId,
) -> Result<(), LirError> {
    // 1. Register the function symbol and its virtual mapping
    let sym = ir.add_function_symbol(symbol_id);
    registry.register_functor(virtual_node_id, sym);

    // 2. Define and register the function skeleton (signature)
    // System functions like total-time/total-cost always have an empty parameter list.
    let skeleton = AtomicFunctionSkeleton::new(
        sym,
        TypedList::empty(),
        Type::primitive(return_type),
    );

    let def_id = ir.add_function_def(skeleton);
    registry.register_function_skeleton(virtual_node_id, def_id);

    Ok(())
}

/// Performs the first pass of the problem encoding by collecting structural definitions.
///
/// This function traverses the AST in preorder to extract non-logical metadata
/// and structural elements required to initialize the [`LiftedProblem`].
///
/// # Responsibilities
///
/// * **Identity**: Extracts and sets the problem name.
/// * **Object Population**: Processes constant and object definitions, ensuring they
///   are registered in both the [`EncodingRegistry`] and the [`LiftedProblem`].
/// * **Offset Management**: Handles the transition between domain constants and
///   problem-specific objects via `set_constant_offset`.
///
/// # Arguments
///
/// * `syntax_tree` - The source AST to be traversed.
/// * `registry` - The registry used to map AST [`NodeId`]s to LIR identifiers.
/// * `ir` - The [`LiftedProblem`] being populated.
///
/// # Errors
///
/// Returns a [`LirError`] if:
/// * An identifier cannot be resolved.
/// * There is a conflict during object registration.
/// * The problem name is malformed.
pub fn collect_problem_definitions(
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    for (node_id, node) in syntax_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, syntax_tree);

        match node.kind() {
            // Set the problem identity
            AstKind::ProblemName => ir.set_problem_name(node.try_ident()?)?,

            // Process objects and constants
            AstKind::ObjectsDef => {
                // Signals the end of domain constants and start of problem objects
                ir.set_constant_offset();
                objects_def::encode(&subtree, registry, ir)?;
            }

            // Other structural nodes can be added here (e.g., Requirements)
            _ => {}
        }
    }
    Ok(())
}

/// Performs the second pass of the problem encoding by transforming logic and expressions.
///
/// This phase evaluates the functional and relational components of the AST. It relies
/// on the mappings established in Phase 1 (Structural Collection) to resolve
/// identifiers into their corresponding LIR entities.
///
/// # Responsibilities
///
/// * **State Initialization**: Encodes the initial state facts and fluent values (`:init`).
/// * **Goal Specification**: Transforms goal conditions into logical formulas (`:goal`).
/// * **Optimization & Metrics**: Encodes plan metrics (e.g., minimizing `total-time`)
///   and problem constraints.
/// * **Hierarchical Planning**: Processes the initial task network for HTN domains.
///
/// # Arguments
///
/// * `syntax_tree` - The source AST to be traversed.
/// * `registry` - The registry containing resolved symbol and skeleton mappings.
/// * `ir` - The [`LiftedProblem`] where the logical specifications are stored.
///
/// # Errors
///
/// Returns a [`LirError`] if:
/// * An expression is semantically invalid.
/// * A symbol used in an expression was not declared in Phase 1.
/// * The HTN initial task network is malformed.
pub fn encode_problem_logic(
    syntax_tree: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    for (node_id, node) in syntax_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, syntax_tree);

        match node.kind() {
            // Initial state: grounded facts and numeric fluents
            AstKind::Init => {
                let init = init::encode(&subtree, registry)?;
                ir.set_init(init);
            }

            // Goal conditions: logical expressions to be satisfied
            AstKind::Goal => {
                let goal_expr = goal::encode(&subtree, registry)?;
                ir.set_goal(goal_expr);
            }

            // Temporal or state-based constraints
            AstKind::Constraints => {
                let constraints = expr::encode(&subtree, registry)?;
                ir.set_problem_constraints(constraints);
            }

            // Optimization objective (e.g., minimize total-time)
            AstKind::Metric => {
                let metric = expr::encode(&subtree, registry)?;
                ir.set_metric_spec(metric);
            }

            // Plan length specifications (often for search guidance)
            AstKind::Length => {
                let length = expr::encode(&subtree, registry)?;
                ir.set_length_spec(length);
            }

            // Initial Task Network for Hierarchical Task Networks (HTN)
            AstKind::InitialTaskNetwork => {
                let network = initial_task_network::encode(&subtree, registry)?;
                ir.set_initial_task_network(network);
            }

            _ => {}
        }
    }
    Ok(())
}
