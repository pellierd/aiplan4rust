//! # Problem-Wide Type Normalization
//!
//! This module orchestrates the complete normalization pass across a [`LiftedProblem`].
//!
//! ## Overview
//! The normalization process is divided into two main phases to ensure data integrity
//! and performance within the Lifted IR (LIR):
//!
//! 1. **Discovery & Propagation**: The entire problem structure is traversed to
//!    identify all unique type signatures. These signatures are unified into
//!    atomic "Pivot" IDs within the [`TypeRegistry`].
//! 2. **Materialization**: Any virtual Pivot ID created during the first phase
//!    is transformed into a formal **anonymous type** within the problem's
//!    official symbol tables.
//!
//! ## Anonymous Types (Pivots)
//! When a variable or parameter uses a composite type (e.g., `(either truck airplane)`),
//! this module generates a stable, deterministic anonymous type (e.g.,
//! `anonymous_either_airplane_truck`).
//!
//! This transformation is essential for the **Grounding engine**, as it replaces
//! complex runtime type-union checks with simple, direct atomic identifier comparisons.
//! By the end of this pass, the LIR is guaranteed to have a flat, non-hierarchical
//! type system where every symbol points to a single canonical [`TypeId`].

use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::typing::registry::TypeRegistry;
use crate::aiplan4rust::compiler::lir::normalization::typing::{
    action, derived_predicate, expr, method, skeleton, typed_list, typed_symbol,
};
use crate::aiplan4rust::compiler::lir::normalization::NormalizationError;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::{Type, TypeId, TypedSymbol};

/// Prefix used for the generation of unified anonymous type symbols.
const ANONYMOUS_PREFIX: &str = "anonymous_either";
/// Separator used between member names in anonymous type symbols.
const ANONYMOUS_SEP: &str = "_";

/// The reserved name for the root of all types in PDDL.
pub const ROOT_TYPE_NAME: &str = "object";

/// The fixed identifier for the root type.
/// Using 0 is optimal for bitsets and array indexing in the grounder.
pub const ROOT_TYPE_ID: TypeId = TypeId::new(0);

/// Normalizes the problem's type hierarchy by resolving composite `either` types
/// into stable, unified atomic identifiers.
///
/// This transformation is a mandatory prerequisite for the Grounding phase. It
/// eliminates overlapping type definitions by ensuring that every variable,
/// object, and formula in the problem refers to a single, canonical atomic
/// [`TypeId`] representing a unique set of primitive leaf types.
///
/// # Parameters
/// * `problem` - A mutable reference to the [`LiftedProblem`] to be transformed.
///
/// # Returns
/// * `Ok(())` if the normalization and remapping process completed successfully.
/// * `Err(LirError)` if a type resolution fails or an internal inconsistency is detected.
///
/// # Implementation Details
/// The process follows a high-performance pipeline designed to minimize memory
/// overhead and ensure structural consistency:
///
/// 1. **Discovery & Resolution**: Traverses the entire problem structure to identify
///    all unique type signatures. The [`TypeRegistry`] unifies identical combinations
///    into single "Pivot" IDs.
/// 2. **Expression Propagation**: Updates all logical formulas, actions, and HTN
///    components to use the new atomic IDs (DFS-based, non-recursive).
/// 3. **Pivot Materialization**: For every newly discovered type signature,
///    injects a formal anonymous type definition and its corresponding symbol
///    into the problem's registries.
///
/// This implementation utilizes reusable buffers ([`Vec`] for tree walking and
/// [`String`] for symbol generation) to minimize heap allocations.
pub fn normalize(
    problem: &mut LiftedProblem,
    store: &mut ExprStore,
    pad: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // Ensure that even untyped problems have a formal 'object' root at ID 0.
    if problem.type_defs().is_empty() {
        create_root_type(problem)?;
    }

    // Initialize the registry with the current state of the problem's type definitions.
    // This registry acts as the source of truth for mapping unions to atomic IDs.
    let mut registry = TypeRegistry::new(problem.type_defs());

    // Pre-allocate buffers to optimize performance during the pass.
    let mut pivot_name_buffer = String::with_capacity(64);

    // --- PHASE 1: DISCOVERY & PROPAGATION ---
    // Traverse all problem components (actions, methods, predicates) and resolve
    // their types. This step populates the registry with any needed anonymous pivots.
    normalize_problem(problem, store, &mut registry, pad)?;

    // --- PHASE 2: MATERIALIZATION ---
    // Inject the new anonymous type definitions into the LiftedProblem.
    // We consume the registry's new type cache to finalize the transformation.
    // This ensures that new TypeIds are backed by real definitions in the LIR.
    for (id, members) in registry.into_new_types() {
        create_anonymous_either_type(problem, id, members, &mut pivot_name_buffer)?;
    }

    Ok(())
}

/// Materializes the root 'object' type at [`ROOT_TYPE_ID`] (0).
///
/// This is the first operation called at the beginning of the normalization pass.
/// It ensures that even in STRIPS-style domains (where no explicit types are
/// defined), there is a valid, atomic type available for all objects and
/// variables to reference.
///
/// # Mechanism
/// 1. **Interning**: The literal string "object" is interned to get a unique symbol ID.
/// 2. **Registration**: The symbol is added to the problem's type registry.
/// 3. **Definiton**: A [`Type::root()`] definition is associated with this ID.
///
/// # Safety and Invariants
/// * **Order Dependence**: This function **must** be called before any other types
///   are registered to guarantee that the root type receives `TypeId(0)`.
/// * **Synchronization**: Uses a `debug_assert_eq!` to catch any desynchronization
///   between the symbol registration and the expected global [`ROOT_TYPE_ID`].
///
/// # Parameters
/// * `problem` - A mutable reference to the [`LiftedProblem`] being initialized.
///
/// # Returns
/// * `Ok(TypeId)` containing the identifier for the root type (always 0).
/// * `Err(LirError)` if the type definition could not be registered.
fn create_root_type(problem: &mut LiftedProblem) -> Result<TypeId, NormalizationError> {
    // 1. Intern the "object" string
    // This provides a consistent symbol name for the root of the hierarchy.
    let name_id = problem
        .interner_mut()
        .intern_symbol(ROOT_TYPE_NAME.to_string());

    // 2. Register the symbol in the problem
    // In a clean LIR state, the first type added must be ID 0.
    let registered_id = problem.add_type_symbol(name_id);

    // Safety Check: Catch logical errors where a type was registered before 'object'.
    debug_assert_eq!(
        registered_id, ROOT_TYPE_ID,
        "Root type 'object' desynchronized. Expected ID 0, got {:?}",
        registered_id
    );

    // 3. Definition Registration
    // The root type is the ultimate parent; in the LIR, it is defined as a
    // Type::root() to stop recursive parent lookups.
    problem.add_type_defs(TypedSymbol::new(registered_id, Type::root()))?;

    Ok(registered_id)
}

/// Materializes a new anonymous `either` type and registers it within the problem.
///
/// This function handles the physical creation of type symbols and their definitions.
/// It generates a deterministic, human-readable name for the new type based on its
/// members (e.g., `__either__airplane__truck`) and ensures the problem's internal
/// symbol registries are updated.
///
/// # Parameters
/// * `problem` - A mutable reference to the [`LiftedProblem`] being transformed.
/// * `id` - The expected [`TypeId`] assigned by the [`TypeRegistry`].
/// * `members` - The [`Type`] definition (collection of member IDs) to materialize.
/// * `name_buffer` - A reusable [`String`] buffer used to construct the type name
///   without repeated heap allocations.
///
/// # Returns
/// * `Ok(TypeId)` - The identifier of the newly registered type.
/// * `Err(LirError)` - If a member name cannot be resolved or registration fails.
///
/// # Determinism & Performance
/// To ensure the transformation is stable across different runs, member names are
/// sorted alphabetically before generating the final symbol name. The use of
/// `name_buffer` minimizes memory pressure during mass type materialization.
fn create_anonymous_either_type(
    problem: &mut LiftedProblem,
    id: TypeId,
    members: Type<TypeId>,
    name_buffer: &mut String,
) -> Result<TypeId, NormalizationError> {
    // --- 1. Symbol Resolution ---
    // Resolve the string representation of each member to construct a composite name.
    let mut member_names: Vec<&str> = Vec::with_capacity(members.len());

    for member_id in members.iter() {
        let string_id = problem.type_symbols().try_get_ident(*member_id)?;
        let name = problem.interner().try_resolve_symbol(*string_id)?;
        member_names.push(name);
    }

    // Sort to ensure determinism (e.g., 'a_b' and 'b_a' always produce the same name).
    member_names.sort_unstable();

    // --- 2. Name Construction ---
    // Build the anonymous identifier in the reusable buffer.
    name_buffer.clear();
    name_buffer.push_str(ANONYMOUS_PREFIX);
    name_buffer.push_str(ANONYMOUS_SEP);

    for (i, name) in member_names.iter().enumerate() {
        if i > 0 {
            name_buffer.push_str(ANONYMOUS_SEP);
        }
        name_buffer.push_str(name);
    }

    // --- 3. Symbol Registration ---
    // Intern the constructed name and register it in the problem's type symbol table.
    let name_id = problem.interner_mut().intern_symbol(name_buffer.clone());

    // Register the symbol and retrieve the allocated TypeId.
    let registered_id = problem.add_type_symbol(name_id);

    // Safety Check: Ensure the registry counter remains synchronized with the problem's state.
    debug_assert_eq!(
        id, registered_id,
        "Desynchronization between TypeRegistry and Problem TypeTable"
    );

    // --- 4. Definition Registration ---
    // Inject the new TypedSymbol (the actual Type definition) into the problem's registry.
    problem.add_type_defs(TypedSymbol::new(registered_id, members))?;

    Ok(registered_id)
}

/// Traverses and updates all components of the [`NewLiftedProblem`] to use unified atomic types.
///
/// Once the type definitions have been analyzed and unified within the [`TypeRegistry`],
/// this function propagates those changes throughout the entire problem hierarchy.
/// It performs in-place mutation on every problem component to ensure that composite
/// `either` type references are replaced by their corresponding atomic identifiers.
///
/// # Arguments
/// * `problem` - A mutable reference to the [`NewLiftedProblem`] to be transformed.
/// * `old` - A mutable reference to the global expression old hosting the node entries.
/// * `registry` - A mutable reference to the [`TypeRegistry`] containing the mapping between
///   composite signatures and their unified atomic [`TypeId`]s.
/// * `pad` - An external, reusable [`Scratchpad`] memory arena tracking traversal states and cache buffers.
///
/// # Returns
/// * `Ok(())` if the entire problem was successfully remapped.
/// * `Err(NormalizationError)` if a type reference in any component fails to resolve within the registry.
///
/// # Propagation Scope
/// This function coordinates the "Ripple Effect" of type normalization across:
/// 1. **Objects & Constants**: Re-typing physical entities in the problem domain.
/// 2. **Skeletons**: Standardizing Predicate, Function, and HTN Task signatures.
/// 3. **Global Logic**: Updating Domain/Problem constraints and Goal conditions.
/// 4. **Operators**: Resolving types in Action/Method parameters, preconditions, and effects.
/// 5. **HTN Structure**: Ensuring the Initial Task Network and Abstract Tasks match the domain.
///
/// # Implementation Detail
/// By passing the exact same `old`, `registry`, and `pad` references down the problem tree,
/// this function ensures absolute type consistency across the scope of all operators while
/// guaranteeing a strict **$\mathcal{O}(1)$ dynamic allocation profile** for the entire pass.
pub fn normalize_problem(
    problem: &mut LiftedProblem,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
    pad: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // --- 1. Objects & Constants ---
    // Re-type all objects and constants to match the unified type space.
    // This is the foundation of the grounding domain.
    for object in problem.object_defs_mut() {
        typed_symbol::normalize_typed_object(object, registry)?;
    }

    // --- 2. Atomic Signatures (Skeletons) ---
    // Update the parameter signatures for all predicates and functions grouped under the skeleton module.
    // This ensures that any subsequent logic referencing these symbols is consistent.
    for atomic_formula in problem.predicate_defs_mut() {
        skeleton::formula::normalize(atomic_formula, store, registry)?;
    }

    for atomic_function in problem.function_defs_mut() {
        skeleton::function::normalize(atomic_function, store, registry)?;
    }

    // --- 3. Constraints & Global Logic ---
    // Perform deep, non-recursive traversals of global expression trees.
    // We copy the IDs to bypass aliasing restrictions and invoke the hash-consed normalizer.
    let current_domain_constraints = problem.domain_constraints();
    let normalized_domain_constraints =
        expr::normalize(current_domain_constraints, store, registry, pad)?;
    problem.set_domain_constraints(normalized_domain_constraints);

    let current_problem_constraints = problem.problem_constraints();
    let normalized_problem_constraints =
        expr::normalize(current_problem_constraints, store, registry, pad)?;
    problem.set_problem_constraints(normalized_problem_constraints);

    // --- 4. HTN Abstract Tasks ---
    // Normalize the parameter interfaces of all abstract tasks in-place.
    for task in problem.task_defs_mut() {
        let current_param_id = task.parameters();
        let normalized_param_id =
            typed_list::normalize_typed_variable_list(current_param_id, store, registry)?;
        task.set_parameters(normalized_param_id);
    }

    // --- 5. Derived Predicates (Axioms) ---
    // Resolve types in both the head (signature) and body (logic) of axioms.
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::normalize(derived_predicate, store, registry, pad)?;
    }

    // --- 6. Primitive Actions ---
    // Resolve types in action parameters, preconditions, and effects.
    // This is often the most computationally intensive part of the normalization.
    for action in problem.action_defs_mut() {
        action::normalize(action, store, registry, pad)?;
    }

    // --- 7. HTN Methods ---
    // Resolve types in method parameters and applicability conditions.
    for method in problem.method_defs_mut() {
        method::normalize(method, store, registry, pad)?;
    }

    // --- 8. Goal State ---
    // Finalize the goal expression tree.
    let current_goal = problem.goal();
    let normalized_goal = expr::normalize(current_goal, store, registry, pad)?;
    problem.set_goal(normalized_goal);

    // --- 9. Initial Task Network ---
    // Ensure the HTN entry point is consistent with the unified type registry.
    let current_param_id = problem.initial_task_network().parameters();
    let normalized_param_id =
        typed_list::normalize_typed_variable_list(current_param_id, store, registry)?;
    problem
        .initial_task_network_mut()
        .set_parameters(normalized_param_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::support::interner::SymbolInterner;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, PredicateSymbolId, VariableId};
    use std::collections::HashSet;

    /// ### Objective
    /// Verify that the flattening process correctly identifies and transforms ad-hoc
    /// `Either` types located inside quantified expressions (Exists/Forall).
    #[test]
    fn test_flatten_quantified_expression_types() -> Result<(), Box<dyn std::error::Error>> {
        let interner = SymbolInterner::new();

        // 1. Initialize the problem (NewLiftedProblem) and its backing stores
        let mut problem = LiftedProblem::new(HashSet::new());
        problem.set_interner(interner);

        let mut store = ExprStore::new();
        let mut pad = Scratchpad::new();

        // 2. Define the base root types
        let sym_a = problem.interner_mut().intern_symbol("a");
        let sym_b = problem.interner_mut().intern_symbol("b");

        let id_a = problem.add_type_symbol(sym_a);
        let id_b = problem.add_type_symbol(sym_b);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 3. Build an expression featuring an ad-hoc "Either" union type
        let mut builder = ExprBuilder::new(&mut store);

        // Define the members [a, b] directly within the variable signature
        let adhoc_members = vec![id_a.as_usize(), id_b.as_usize()];

        // Create the variable ?X (ID 1) bound to this composite typing
        let var_x = builder.typed_variable(1, &adhoc_members);
        let list_x = builder.typed_variable_list(vec![var_x]);

        // To prevent the variable from being pruned, it must be referenced inside the body
        let arg_x = builder.variable(VariableId::from(1));

        // Mock identifiers for the predicate P(?X)
        let pred_sym = PredicateSymbolId::from(3);
        let skel_id = AtomSkeletonId::from(100);

        // Expression body: P(?X)
        let atomic_p = builder.atomic_formula(pred_sym, &[arg_x], skel_id);
        let exists_x = builder.exists(list_x, atomic_p)?;

        // Inject the finalized expression into the problem constraints
        problem.set_problem_constraints(exists_x);

        // --- 4. EXECUTE STEP-BY-STEP FLATTENING PASS ---
        normalize(&mut problem, &mut store, &mut pad)?;

        // --- 5. VERIFICATIONS ---
        let final_expr = problem.problem_constraints();

        // --- BORROW CHECKER DECONFLICTION ---
        // Extract the internal `TypedListId` into an isolated, copyable local variable.
        // This allows the immutable borrow on `store` (via `store.get`) to strictly die
        // before we query the store or problem structures downstream.
        let target_vars_id = if let Some(node) = store.get(final_expr) {
            if let ExprKind::ExistsNew(vars_id) = node.kind() {
                Some(*vars_id)
            } else {
                None
            }
        } else {
            None
        };

        // Extract and validate the quantifier's bound variables using the isolated ID
        if let Some(vars_id) = target_vars_id {
            // Now that `store` is unborrowed, fetch the actual flattened list from it
            let actual_vars = store.fetch_typed_list(vars_id).unwrap();

            assert_eq!(actual_vars.len(), 1, "Should have exactly 1 variable");
            let var_type = actual_vars[0].ty();

            // Verification 1: The ad-hoc typing [a, b] must be replaced by a single unique ID (length 1)
            assert_eq!(
                var_type.members().len(),
                1,
                "The type within the quantifier must be a redirection to the newly materialized atomic type"
            );

            // Verification 2: Retrieve the definition of the newly created materialized type
            let new_type_id = var_type.members()[0];
            let new_type_def = problem.try_get_type(new_type_id)?.ty();

            // Verification 3: The materialized type must encapsulate the original root members
            assert!(
                new_type_def.members().contains(&id_a),
                "The new materialized type must contain member 'a'"
            );
            assert!(
                new_type_def.members().contains(&id_b),
                "The new materialized type must contain member 'b'"
            );

            // Verification 4: The generated name must be deterministic and use the anonymous prefix
            let sym_id = problem.type_symbols().try_get_ident(new_type_id)?;
            let final_name = problem.interner().try_resolve_symbol(*sym_id)?;

            assert!(final_name.starts_with(ANONYMOUS_PREFIX));
            assert!(final_name.contains("a"));
            assert!(final_name.contains("b"));
        } else {
            panic!("Resulting expression root is not an Exists node or could not be found !");
        }

        Ok(())
    }
}
