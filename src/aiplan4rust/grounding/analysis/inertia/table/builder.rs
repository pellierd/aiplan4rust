//! Inertia Analysis for Lifted Planning Problems.
//!
//! This module identifies the "stability" of predicates and functions within a
//! [`LiftedProblem`]. By determining which elements are constant (static) and
//! which are subject to change (fluents), the planner can perform significantly
//! more efficient grounding and state-space pruning.
//!
//! ### Derived Predicates Simplification
//! Currently, this analyzer implements a **conservative simplification**: all
//! derived predicates are automatically categorized as [`Inertia::Fluent`].
//!
//! > **Note on Potential Optimization:** In theory, a derived predicate could
//! > be categorized as [`Inertia::Static`] if, and only if, all predicates
//! > appearing in its body (definition) are themselves static.
//! >
//! > Implementing this recursive check would allow for even more aggressive
//! > pruning during the search phase. However, in the current implementation,
//! > we treat them as fluents for simplicity and safety, avoiding the need for
//! > a dependency graph analysis of axioms.

use std::collections::HashSet;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId};
use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
use crate::aiplan4rust::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::analysis::inertia::table::InertiaTableError;

/// Analyzes a lifted planning problem to determine the inertia of all predicates and functions.
///
/// This is the entry point of the inertia analysis module. It orchestrates a multi-pass
/// scan of the problem's actions and initial state to categorize symbols into three
/// categories:
/// - **Fluent**: Symbols that can change their value/truth over time (via effects or TILs).
/// - **Static Positive**: Symbols that are initialized and never modified.
/// - **Static Negative**: Symbols that are never initialized and never modified (dead facts).
///
/// # Arguments
///
/// * `problem` - A reference to the [`LiftedProblem`] containing domain and problem definitions.
///
/// # Returns
///
/// Returns a [`Result`] containing the populated [`InertiaTable`] on success.
///
/// # Errors
///
/// Returns a [`LirError`] if any expression tree traversal (actions or initial state) fails,
/// typically due to a malformed AST or an inaccessible node.
pub fn build(problem: &LiftedProblem) -> Result<InertiaTable, InertiaTableError> {
    let mut fluent_predicates = HashSet::new();
    let mut fluent_functions = HashSet::new();
    let mut static_predicates = HashSet::new();
    let mut static_functions = HashSet::new();

    // Step 1: Identify symbols modified by action effects (Fluents)
    collect_all_action_fluents(problem, &mut fluent_predicates, &mut fluent_functions)?;

    // Step 2: Scan initial state for static facts and Timed Initial Literals (TILs)
    // Note: TILs will add symbols to the fluent sets.
    collect_initial_facts(
        problem.init(),
        &mut static_predicates,
        &mut static_functions,
        &mut fluent_predicates,
        &mut fluent_functions,
    )?;

    // Step 3: Build and return the final categorization table
    Ok(build_inertia_table(
        problem,
        fluent_predicates,
        fluent_functions,
        static_predicates,
        static_functions,
    ))
}

/// Scans all action definitions (standard and durative) to identify modified predicates and functions.
///
/// This function iterates through every action's effects to populate the sets of fluents,
/// which are essential for determining the inertia of the problem's components.
///
/// # Arguments
///
/// * `problem` - The lifted problem containing all action and durative action definitions.
/// * `fluent_predicates` - A mutable set to be populated with IDs of predicates modified by any action.
/// * `fluent_functions` - A mutable set to be populated with IDs of numeric functions modified by any action.
///
/// # Errors
///
/// Returns a [`LirError`] if an error occurs while traversing an action's effect expression.
fn collect_all_action_fluents(
    problem: &LiftedProblem,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    // Collect fluents from standard instantaneous actions
    for action in problem.action_defs() {
        collect_fluents_from_effect(
            action.effect(),
            fluent_predicates,
            fluent_functions
        )?;
    }

    Ok(())
}

/// Builds the final [`InertiaTable`] by categorizing every predicate and function
/// definition found in the problem.
///
/// This function aggregates results from effect analysis and initial state scanning
/// to assign an [`Inertia`] status to each element based on its presence (or absence)
/// in action effects and the initial state.
///
/// # Arguments
///
/// * `problem` - The lifted problem containing all original definitions.
/// * `fluent_predicates` - Set of predicate IDs modified by actions or TILs.
/// * `fluent_functions` - Set of function IDs modified by effects or TILs.
/// * `constant_predicates` - Set of predicate IDs present in the initial state but never modified.
/// * `constant_functions` - Set of function IDs initialized but never modified.
///
/// # Returns
///
/// A populated [`InertiaTable`] representing the stability of all symbols.
fn build_inertia_table(
    problem: &LiftedProblem,
    fluent_predicates: HashSet<AtomSkeletonId>,
    fluent_functions: HashSet<FunctionSkeletonId>,
    constant_predicates: HashSet<AtomSkeletonId>,
    constant_functions: HashSet<FunctionSkeletonId>,
) -> InertiaTable {
    let mut table = InertiaTable::empty();

    // Les prédicats dérivés sont calculés par des axiomes, donc considérés comme Fluents.
    let derived_ids: HashSet<_> = problem
        .derived_predicate_defs()
        .iter()
        .map(|d| d.header_id())
        .collect();

    // --- Catégorisation des Prédicats ---
    for (idx, _) in problem.predicate_defs().iter().enumerate() {
        let id = AtomSkeletonId::from(idx);

        let inertia = if derived_ids.contains(&id) || fluent_predicates.contains(&id) {
            // Le prédicat change de valeur (ADD et/ou DEL présents).
            Inertia::fluent()
        } else if constant_predicates.contains(&id) {
            // Le prédicat est présent dans l'init et n'est JAMAIS modifié.
            // Dans IPP, c'est l'Inertie Positive ET Négative.
            Inertia::positive_negative()
        } else {
            // Le prédicat n'est ni dans les effets, ni dans l'état initial.
            // Il est donc "Inerte Positif" (ne peut pas être ajouté) et reste absent (False).
            Inertia::negative()
        };
        table.insert_predicate(id, inertia);
    }

    // --- Catégorisation des Fonctions (Numériques) ---
    for (idx, _) in problem.function_defs().iter().enumerate() {
        let id = FunctionSkeletonId::from(idx);

        let inertia = if fluent_functions.contains(&id) {
            // La fonction est modifiée par des effets (assign, increase, decrease).
            Inertia::fluent()
        } else if constant_functions.contains(&id) {
            // La fonction est définie à l'initialisation et reste immuable.
            Inertia::positive_negative()
        } else {
            // La fonction n'est ni dans les effets, ni initialisée.
            // Elle est considérée Inerte Négative (sa valeur par défaut, souvent 0, ne peut être diminuée).
            Inertia::negative()
        };
        table.insert_function(id, inertia);
    }

    table
}

/// Traverses an effect expression to identify modified predicates and functions (Fluents).
///
/// This function performs a depth-first search through the effect tree. It specifically
/// handles conditional effects (`When` nodes) by only traversing the effect branch,
/// as the condition branch is considered a read-only context.
///
/// # Arguments
///
/// * `logic` - The effect expression to analyze.
/// * `fluent_predicates` - A set to be populated with IDs of modified predicates.
/// * `fluent_functions` - A set to be populated with IDs of modified numeric functions.
///
/// # Errors
///
/// Returns a [`LirError`] if the expression tree is malformed or if a node cannot be accessed.
pub fn collect_fluents_from_effect(
    expr: &Expr,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    // Early exit if the expression is empty
    if expr.is_empty() {
        return Ok(());
    }

    let mut stack = vec![expr.try_root_id()?];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;

        match node.content() {
            // Predicates found in an effect context are marked as Fluent
            ExprContent::AtomSkeleton(id) => {
                fluent_predicates.insert(*id);
            }

            // Functions found in an effect context (e.g., assignments) are marked as Fluent
            ExprContent::FunctionSkeleton(id) => {
                fluent_functions.insert(*id);
            }

            _ => {
                // Special handling for Conditional Effects (When nodes)
                // In PDDL/HTN, (when <condition> <effect>):
                // - Child 0 (condition) is read-only (Static or Fluent, but not modified here).
                // - Child 1 (effect) contains the actual modifications.
                if node.kind() == ExprKind::When {
                    if let Some(&effect_id) = node.children().get(1) {
                        stack.push(effect_id);
                    }
                } else {
                    // Standard recursive traversal for structural nodes (And, Not, Forall, etc.)
                    for &child_id in node.children().iter().rev() {
                        stack.push(child_id);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Traverses the initial state expression to categorize predicates and functions based on their inertia.
///
/// It distinguishes between constant initial facts (Positive inertia) and
/// Timed Initial Literals (TILs), which are treated as Fluents because they change over time.
///
/// # Arguments
///
/// * `init_expr` - The expression representing the problem's initial state.
/// * `static_predicates` - A set to store IDs of predicates that remain constant.
/// * `static_functions` - A set to store IDs of numeric functions that remain constant.
/// * `fluent_predicates` - A set to store IDs of predicates modified by TILs or actions.
/// * `fluent_functions` - A set to store IDs of functions modified by TILs or effects.
///
/// # Errors
///
/// Returns a [`LirError`] if the expression tree is malformed or if a node cannot be accessed.
pub fn collect_initial_facts(
    init_expr: &Expr,
    static_predicates: &mut HashSet<AtomSkeletonId>,
    static_functions: &mut HashSet<FunctionSkeletonId>,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    // Early exit if the expression tree is empty
    if init_expr.is_empty() {
        return Ok(());
    }

    // Stack stores pairs of (NodeID, is_within_temporal_context)
    let mut stack = vec![(init_expr.try_root_id()?, false)];

    while let Some((node_id, is_timed)) = stack.pop() {
        let node = init_expr.try_node(node_id)?;

        // A node is considered "timed" if it is already inside a TIL block
        // or if it specifically defines a Timed Initial Literal.
        let within_til = is_timed || node.kind() == ExprKind::TimedInitialLiteral;

        match node.content() {
            // Predicates: categorized as Fluent if inside a TIL, otherwise Static (Positive).
            ExprContent::AtomSkeleton(id) if within_til => { fluent_predicates.insert(*id); }
            ExprContent::AtomSkeleton(id) => { static_predicates.insert(*id); }

            // Functions: categorized as Fluent if inside a TIL, otherwise Static.
            ExprContent::FunctionSkeleton(id) if within_til => { fluent_functions.insert(*id); }
            ExprContent::FunctionSkeleton(id) => { static_functions.insert(*id); }

            // For structural nodes (And, Not, etc.), propagate the temporal context to children.
            _ => {
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, within_til));
                }
            }
        }
    }

    Ok(())
}
