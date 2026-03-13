//! # Problem-Wide Type Flattening
//!
//! This module orchestrates the complete flattening pass across a [`LiftedProblem`].
//!
//! ## Overview
//! The flattening process is divided into two main phases to ensure data integrity
//! and performance:
//!
//! 1. **Discovery & Propagation**: The entire problem structure is traversed to
//!    identify all unique type signatures. These signatures are unified into
//!    "Pivot" IDs within the [`TypeRegistry`].
//! 2. **Materialization**: Any virtual Pivot ID created during the first phase
//!    is transformed into a formal **anonymous type** within the problem's
//!    official symbol tables.
//!
//! ## Anonymous Types (Pivots)
//! When a variable or parameter uses a composite type (e.g., `(either truck airplane)`),
//! this module generates a stable, deterministic anonymous type (e.g.,
//! `anonymous_either_airplane_truck`). This ensures that the grounding engine
//! sees a flat, non-hierarchical type system.

use crate::aiplan4rust::lang::{Type, TypeId, TypedSymbol};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::{
    atomic_formula_skeleton, atomic_function_skeleton, derived_predicate,
    expr, typed_symbol, task, action, method, initial_task_network
};
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::lir::passes::either_type::registry::TypeRegistry;

/// Prefix used for the generation of unified anonymous type symbols.
const ANONYMOUS_PREFIX: &str = "anonymous_either";
/// Separator used between member names in anonymous type symbols.
const ANONYMOUS_SEP: &str = "_";

/// Flattens the problem's type hierarchy by resolving composite `either` types
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
/// * `Ok(())` if the flattening and remapping process completed successfully.
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
pub fn flatten(problem: &mut LiftedProblem) -> Result<(), LirError> {
    // Initialize the registry with the current state of the problem's type definitions.
    let mut registry = TypeRegistry::new(problem.type_defs());

    // Pre-allocate buffers to optimize performance during the pass.
    let mut pivot_name_buffer = String::with_capacity(64);
    let mut stack = Vec::with_capacity(64);

    // --- PHASE 1: DISCOVERY & PROPAGATION ---
    // Traverse all problem components (actions, methods, predicates) and resolve
    // their types. This step populates the registry with any needed anonymous pivots.
    flatten_problem(problem, &mut registry, &mut stack)?;

    // --- PHASE 2: MATERIALIZATION ---
    // Inject the new anonymous type definitions into the LiftedProblem.
    // We consume the registry's new type cache to finalize the transformation.
    for (id, members) in registry.into_new_types() {
        create_anonymous_either_type(problem, id, members, &mut pivot_name_buffer)?;
    }

    Ok(())
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
) -> Result<TypeId, LirError> {
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
    debug_assert_eq!(id, registered_id, "Desynchronization between TypeRegistry and Problem TypeTable");

    // --- 4. Definition Registration ---
    // Inject the new TypedSymbol (the actual Type definition) into the problem's registry.
    problem.add_type_defs(TypedSymbol::new(
        registered_id,
        members
    ))?;

    Ok(registered_id)
}

/// Traverses and updates all components of the [`LiftedProblem`] to use unified atomic types.
///
/// Once the type definitions have been analyzed and unified within the [`TypeRegistry`],
/// this function propagates those changes throughout the entire problem hierarchy.
/// It performs in-place mutation on every problem component to ensure that composite
/// `either` type references are replaced by their corresponding atomic identifiers.
///
/// # Parameters
/// * `problem` - A mutable reference to the [`LiftedProblem`] to be transformed.
/// * `registry` - The [`TypeRegistry`] containing the mapping between composite signatures
///   and their unified atomic [`TypeId`]s.
/// * `stack` - A reusable [`Vec<NodeId>`] buffer used for efficient, non-recursive
///   traversal of expression trees across actions, methods, and constraints.
///
/// # Returns
/// * `Ok(())` if the entire problem was successfully remapped.
/// * `Err(LirError)` if a type reference in any component fails to resolve within the registry.
///
/// # Propagation Scope
/// This function coordinates the "Ripple Effect" of type flattening across:
/// 1. **Objects & Constants**: Re-typing physical entities in the problem domain.
/// 2. **Skeletons**: Standardizing Predicate, Function, and HTN Task signatures.
/// 3. **Global Logic**: Updating Domain/Problem constraints and Goal conditions.
/// 4. **Operators**: Resolving types in Action/Method parameters, preconditions, and effects.
fn flatten_problem(
    problem: &mut LiftedProblem,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {

    // --- Objects & Constants ---
    // Re-type all objects and constants to match the unified type space.
    for object in problem.object_defs_mut() {
        typed_symbol::flatten_typed_object(object, registry)?;
    }

    // --- Atomic Signatures (Skeletons) ---
    // Update the parameter signatures for all predicates and functions.
    for atomic_formula in problem.predicate_defs_mut() {
        atomic_formula_skeleton::flatten(atomic_formula, registry)?;
    }

    for atomic_function in problem.function_defs_mut() {
        atomic_function_skeleton::flatten(atomic_function, registry)?;
    }

    // --- Constraints & Global Logic ---
    // Perform deep, non-recursive traversals of global expression trees.
    expr::flatten(problem.domain_constraints_mut(), registry, stack)?;
    expr::flatten(problem.problem_constraints_mut(), registry, stack)?;

    // --- HTN Abstract Tasks ---
    // Update abstract task definitions within the HTN hierarchy.
    for task in problem.task_defs_mut() {
        task::flatten(task, registry)?;
    }

    // --- Derived Predicates (Axioms) ---
    // Resolve types in both the head (signature) and body (logic) of axioms.
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::flatten(derived_predicate, registry, stack)?;
    }

    // --- Primitive Actions ---
    // Resolve types in action parameters, preconditions, and effects.
    for action in problem.action_defs_mut() {
        action::flatten(action, registry, stack)?;
    }

    // --- HTN Methods ---
    // Resolve types in method parameters and applicability conditions.
    for method in problem.method_defs_mut() {
        method::flatten(method, registry, stack)?;
    }

    // --- Goal State ---
    // Finalize the goal expression tree.
    expr::flatten(problem.goal_mut(), registry, stack)?;

    // --- Initial Task Network ---
    // Ensure the HTN entry point is consistent with the unified type registry.
    initial_task_network::flatten(problem.initial_task_network_mut(), registry)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*; // Importe flatten et create_anonymous_either_type
    use crate::aiplan4rust::interner::SymbolInterner;
    use crate::aiplan4rust::lang::{Type, TypeId, TypedSymbol};
    use crate::aiplan4rust::lir::problem::LiftedProblem;
    use crate::aiplan4rust::lir::expr::ExprBuilder;
    use std::collections::HashSet;
    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly identifies and transforms ad-hoc
    /// `Either` types located inside quantified expressions (Exists/Forall).
    fn test_flatten_quantified_expression_types() -> Result<(), Box<dyn std::error::Error>> {
        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Définition des types de base (Roots)
        let sym_a = problem.interner_mut().intern_symbol("a");
        let sym_b = problem.interner_mut().intern_symbol("b");

        let id_a = problem.add_type_symbol(sym_a);
        let id_b = problem.add_type_symbol(sym_b);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 2. Construction d'une expression avec un either_type "Either" ad-hoc
        let mut builder = ExprBuilder::new();

        // On définit les membres [a, b] directement dans la variable
        let adhoc_members = vec![id_a.as_usize(), id_b.as_usize()];

        // Création de la variable ?X associée à ce either_type composite
        let var_x = builder.typed_variable(1, &adhoc_members);
        let list_x = builder.typed_variable_list(vec![var_x]);

        // Corps de l'expression : P()
        let atomic_p = builder.atomic_formula(3, vec![]);
        let exists_x = builder.exists(list_x, atomic_p);

        builder.set_root(exists_x)?;
        let expr = builder.finish();

        // Injection de l'expression dans les contraintes du problème
        problem.set_problem_constraints(expr);

        // --- 3. EXECUTION DU FLATTEN ---
        // Phase 1 (Collecte dans l'expression) + Phase 2 (Matérialisation globale)
        flatten(&mut problem)?;

        // --- 4. VERIFICATIONS ---
        let final_expr = problem.problem_constraints();

        // On récupère les variables du quantificateur après transformation
        let vars = final_expr.try_root_node()?.content().try_quantifier_vars()?;
        let var_type = vars[0].ty();

        // Verification 1 : Le either_type ad-hoc [a, b] doit avoir été remplacé par un ID unique (longueur 1)
        assert_eq!(
            var_type.members().len(),
            1,
            "Le type dans le quantificateur doit être une redirection vers le nouveau type matérialisé"
        );

        // Verification 2 : On récupère la définition du nouveau either_type créé
        let new_type_id = var_type.members()[0];
        let new_type_def = problem.try_get_type(new_type_id)?.ty();

        // Verification 3 : Le nouveau either_type doit contenir les racines originales
        assert!(new_type_def.members().contains(&id_a), "Le nouveau type doit contenir 'a'");
        assert!(new_type_def.members().contains(&id_b), "Le nouveau type doit contenir 'b'");

        // Verification 4 : Le nom doit être déterministe et utiliser ton préfixe
        let sym_id = problem.type_symbols().try_get_ident(new_type_id)?;
        let final_name = problem.interner().try_resolve_symbol(*sym_id)?;

        assert!(final_name.starts_with(ANONYMOUS_PREFIX));
        // Le nom doit contenir 'a' et 'b' (l'ordre dépend de ton tri alphabétique)
        assert!(final_name.contains("a"));
        assert!(final_name.contains("b"));

        Ok(())
    }
}
