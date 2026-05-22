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

use crate::aiplan4rust::lang::{Type, TypeId, TypedSymbol};
use crate::aiplan4rust::lir::store::passes::typing::registry::TypeRegistry;
use crate::aiplan4rust::lir::store::passes::typing::{
    action, atomic_formula_skeleton, atomic_function_skeleton, derived_predicate, expr,
    initial_task_network, method, task, typed_symbol,
};
use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::NodeId;

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
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), LirError> {
    // Ensure that even untyped problems have a formal 'object' root at ID 0.
    if problem.type_defs().is_empty() {
        create_root_type(problem)?;
    }

    // Initialize the registry with the current state of the problem's type definitions.
    // This registry acts as the source of truth for mapping unions to atomic IDs.
    let mut registry = TypeRegistry::new(problem.type_defs());

    // Pre-allocate buffers to optimize performance during the pass.
    let mut pivot_name_buffer = String::with_capacity(64);
    let mut stack = Vec::with_capacity(64);

    // --- PHASE 1: DISCOVERY & PROPAGATION ---
    // Traverse all problem components (actions, methods, predicates) and resolve
    // their types. This step populates the registry with any needed anonymous pivots.
    normalize_problem(problem, &mut registry, &mut stack)?;

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
fn create_root_type(problem: &mut LiftedProblem) -> Result<TypeId, LirError> {
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
    debug_assert_eq!(
        id, registered_id,
        "Desynchronization between TypeRegistry and Problem TypeTable"
    );

    // --- 4. Definition Registration ---
    // Inject the new TypedSymbol (the actual Type definition) into the problem's registry.
    problem.add_type_defs(TypedSymbol::new(registered_id, members))?;

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
/// This function coordinates the "Ripple Effect" of type normalization across:
/// 1. **Objects & Constants**: Re-typing physical entities in the problem domain.
/// 2. **Skeletons**: Standardizing Predicate, Function, and HTN Task signatures.
/// 3. **Global Logic**: Updating Domain/Problem constraints and Goal conditions.
/// 4. **Operators**: Resolving types in Action/Method parameters, preconditions, and effects.
/// 5. **HTN Structure**: Ensuring the Initial Task Network and Abstract Tasks match the domain.
fn normalize_problem(
    problem: &mut LiftedProblem,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // --- 1. Objects & Constants ---
    // Re-type all objects and constants to match the unified type space.
    // This is the foundation of the grounding domain.
    for object in problem.object_defs_mut() {
        typed_symbol::normalize_typed_object(object, registry)?;
    }

    // --- 2. Atomic Signatures (Skeletons) ---
    // Update the parameter signatures for all predicates and functions.
    // This ensures that any subsequent logic referencing these symbols is consistent.
    for atomic_formula in problem.predicate_defs_mut() {
        atomic_formula_skeleton::normalize(atomic_formula, registry)?;
    }

    for atomic_function in problem.function_defs_mut() {
        atomic_function_skeleton::normalize(atomic_function, registry)?;
    }

    // --- 3. Constraints & Global Logic ---
    // Perform deep, non-recursive traversals of global expression trees.
    expr::normalize(problem.domain_constraints_mut(), registry, stack)?;
    expr::normalize(problem.problem_constraints_mut(), registry, stack)?;

    // --- 4. HTN Abstract Tasks ---
    // Update abstract task definitions within the HTN hierarchy.
    for task in problem.task_defs_mut() {
        task::normalize(task, registry)?;
    }

    // --- 5. Derived Predicates (Axioms) ---
    // Resolve types in both the head (signature) and body (logic) of axioms.
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::normalize(derived_predicate, registry, stack)?;
    }

    // --- 6. Primitive Actions ---
    // Resolve types in action parameters, preconditions, and effects.
    // This is often the most computationally intensive part of the normalization.
    for action in problem.action_defs_mut() {
        action::normalize(action, registry, stack)?;
    }

    // --- 7. HTN Methods ---
    // Resolve types in method parameters and applicability conditions.
    for method in problem.method_defs_mut() {
        method::normalize(method, registry, stack)?;
    }

    // --- 8. Goal State ---
    // Finalize the goal expression tree.
    expr::normalize(problem.goal_mut(), registry, stack)?;

    // --- 9. Initial Task Network ---
    // Ensure the HTN entry point is consistent with the unified type registry.
    initial_task_network::normalize(problem.initial_task_network_mut(), registry)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // Importe flatten et create_anonymous_either_type
    use crate::aiplan4rust::interner::SymbolInterner;
    use crate::aiplan4rust::lang::{Type, TypedSymbol};
    use crate::aiplan4rust::lir::store::expr_old::ExprBuilder;
    use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
    use std::collections::HashSet;
    #[test]
    /// ### Objective
    /// Verify that the flattening process correctly identifies and transforms ad-hoc
    /// `Either` types located inside quantified expressions (Exists/Forall).
    fn test_flatten_quantified_expression_types() -> Result<(), Box<dyn std::error::Error>> {
        let interner = SymbolInterner::new();
        let mut problem = LiftedProblem::new(HashSet::new());
        problem.set_interner(interner);

        // 1. Définition des types de base (Roots)
        let sym_a = problem.interner_mut().intern_symbol("a");
        let sym_b = problem.interner_mut().intern_symbol("b");

        let id_a = problem.add_type_symbol(sym_a);
        let id_b = problem.add_type_symbol(sym_b);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;

        // 2. Construction d'une expression avec un typing "Either" ad-hoc
        let mut builder = ExprBuilder::new();

        // On définit les membres [a, b] directement dans la variable
        let adhoc_members = vec![id_a.as_usize(), id_b.as_usize()];

        // Création de la variable ?X associée à ce typing composite
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
        normalize(&mut problem)?;

        // --- 4. VERIFICATIONS ---
        let final_expr = problem.problem_constraints();

        // On récupère les variables du quantificateur après transformation
        let vars = final_expr
            .try_root_node()?
            .content()
            .try_quantifier_vars()?;
        let var_type = vars[0].ty();

        // Verification 1 : Le typing ad-hoc [a, b] doit avoir été remplacé par un ID unique (longueur 1)
        assert_eq!(
            var_type.members().len(),
            1,
            "Le type dans le quantificateur doit être une redirection vers le nouveau type matérialisé"
        );

        // Verification 2 : On récupère la définition du nouveau typing créé
        let new_type_id = var_type.members()[0];
        let new_type_def = problem.try_get_type(new_type_id)?.ty();

        // Verification 3 : Le nouveau typing doit contenir les racines originales
        assert!(
            new_type_def.members().contains(&id_a),
            "Le nouveau type doit contenir 'a'"
        );
        assert!(
            new_type_def.members().contains(&id_b),
            "Le nouveau type doit contenir 'b'"
        );

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
