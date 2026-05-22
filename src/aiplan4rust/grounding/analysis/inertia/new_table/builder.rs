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

use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
use crate::aiplan4rust::grounding::analysis::inertia::new_table::InertiaTable;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId};
use crate::aiplan4rust::lir::expr::expr::Expr;
use crate::aiplan4rust::lir::expr::ExprEntryKind;
use crate::aiplan4rust::lir::problem::NewLiftedProblem;
use crate::analysis::inertia::new_table::InertiaTableError;
use std::collections::HashSet;

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
pub fn build(problem: &NewLiftedProblem) -> Result<InertiaTable, InertiaTableError> {
    let mut fluent_predicates = HashSet::new();
    let mut fluent_functions = HashSet::new();
    let mut static_predicates = HashSet::new();
    let mut static_functions = HashSet::new();

    // Step 1: Identify symbols modified by action effects (Fluents)
    collect_all_action_fluents(problem, &mut fluent_predicates, &mut fluent_functions)?;

    // Step 2: Scan initial state for static facts and Timed Initial Literals (TILs)
    // Note: TILs will add symbols to the fluent sets.
    let init = Expr::new(problem.init(), problem.store());
    collect_initial_facts(
        init,
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
    problem: &NewLiftedProblem,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    // Collect fluents from standard instantaneous actions
    for action in problem.action_defs() {
        let effects = Expr::new(action.effect(), problem.store());
        collect_fluents_from_effect(effects, fluent_predicates, fluent_functions)?;
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
    problem: &NewLiftedProblem,
    fluent_predicates: HashSet<AtomSkeletonId>,
    fluent_functions: HashSet<FunctionSkeletonId>,
    constant_predicates: HashSet<AtomSkeletonId>,
    constant_functions: HashSet<FunctionSkeletonId>,
) -> InertiaTable {
    let mut table = InertiaTable::empty();

    // --- Predicate Categorization ---
    for (idx, _) in problem.predicate_defs().iter().enumerate() {
        let id = AtomSkeletonId::from(idx);

        // Utilize the helper method from the Problem to check for axioms
        let inertia = if problem.is_derived_predicate(id) || fluent_predicates.contains(&id) {
            // Predicate changes value (via action effects, TILs, or Axioms).
            Inertia::fluent()
        } else if constant_predicates.contains(&id) {
            // Predicate is present in the initial state and is NEVER modified.
            Inertia::positive_negative()
        } else {
            // Predicate is neither in effects nor in the initial state -> Always False.
            Inertia::negative()
        };
        table.insert_predicate(id, inertia);
    }

    // --- Function Categorization (Numeric) ---
    for (idx, _) in problem.function_defs().iter().enumerate() {
        let id = FunctionSkeletonId::from(idx);

        let inertia = if fluent_functions.contains(&id) {
            // Function is modified by effects (assign, increase, decrease, etc.).
            Inertia::fluent()
        } else if constant_functions.contains(&id) {
            // Function is defined at initialization and remains immutable.
            Inertia::positive_negative()
        } else {
            // Function is neither initialized nor modified.
            // Considered Negative Inertia (its default value, usually 0, cannot be changed).
            Inertia::negative()
        };
        table.insert_function(id, inertia);
    }

    table
}

/// Traverses an effect expression to identify modified predicates and functions (Fluents).
///
/// This function performs an iterative preorder traversal of the effect tree.
/// It specifically handles conditional effects (`When` nodes) by skipping the
/// condition branch, as conditions are read-only contexts and do not produce fluents.
///
/// # Arguments
///
/// * `expr_old` - The effect expression to analyze.
/// * `fluent_predicates` - A mutable set to be populated with IDs of modified predicates.
/// * `fluent_functions` - A mutable set to be populated with IDs of modified numeric functions.
///
/// # Errors
///
/// Returns an [`InertiaTableError`] if the expression store cannot be accessed or
/// if the tree structure is invalid.
fn collect_fluents_from_effect(
    expr: Expr<'_>,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    let mut it = expr.store().tree_preorder(expr.root_id());

    while let Some((_id, _depth, _is_last, entry)) = it.next() {
        match entry.kind() {
            ExprEntryKind::AtomicFormula(id) => {
                fluent_predicates.insert(*id);
            }

            ExprEntryKind::Function(id) => {
                fluent_functions.insert(*id);
            }

            ExprEntryKind::When => {
                // A 'When' node has exactly 2 children: [0: Condition, 1: Effect].
                // The iterator's internal stack has pushed them as: [Effect, Condition] <- Top.
                // We skip the first child (Condition) because it contains no mutations.
                it.skip_children(1);

                // The subsequent it.next() call will proceed directly to the Effect branch.
            }

            // Other structural nodes (And, etc.) are traversed normally by the iterator.
            _ => {}
        }
    }
    Ok(())
}

/// Traverses the initial state expression to categorize predicates and functions based on their inertia.
///
/// It distinguishes between constant initial facts (Positive/Negative inertia) and
/// Timed Initial Literals (TILs), which are treated as Fluents because they change
/// values at specific time points.
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
/// Returns an [`InertiaTableError`] if the expression tree is malformed or if
/// a node cannot be accessed.
fn collect_initial_facts(
    init_expr: Expr<'_>,
    static_predicates: &mut HashSet<AtomSkeletonId>,
    static_functions: &mut HashSet<FunctionSkeletonId>,
    fluent_predicates: &mut HashSet<AtomSkeletonId>,
    fluent_functions: &mut HashSet<FunctionSkeletonId>,
) -> Result<(), InertiaTableError> {
    // Stack of (NodeId, is_within_temporal_context).
    // We use an explicit stack to avoid recursion and stay memory-efficient.
    let mut stack = vec![(init_expr.root_id(), false)];

    while let Some((node_id, is_timed)) = stack.pop() {
        // Fetch the node from the store associated with the expression handle.
        let node = init_expr.store().fetch(node_id)?;

        // Determine if the current context is temporal (within a TIL).
        let within_til = is_timed || matches!(node.kind(), ExprEntryKind::TimedInitialLiteral);

        match node.kind() {
            // --- Predicates categorization ---
            ExprEntryKind::AtomicFormula(id) => {
                if within_til {
                    fluent_predicates.insert(*id);
                } else {
                    static_predicates.insert(*id);
                }
            }

            // --- Functions categorization ---
            ExprEntryKind::Function(id) => {
                if within_til {
                    fluent_functions.insert(*id);
                } else {
                    static_functions.insert(*id);
                }
            }

            // --- Structural Nodes (And, Or, Not, etc.) ---
            _ => {
                // Push children onto the stack while propagating the temporal context.
                // We iterate in reverse to maintain the natural reading order
                // (though order is irrelevant for inertia analysis).
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, within_til));
                }
            }
        }
    }

    Ok(())
}
