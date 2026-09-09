//! Variable Aliasing and Equality Unification Module.
//!
//! This module provides a high-performance, stack-allocated, zero-allocation
//! system to extract, track, and flatten equality constraints (aliases) between logical
//! variables and constants within an expression AST.
//!
//! # Architecture & Performance Highlights
//!
//! * **Zero Heap Allocation**: Utilizing a pre-allocated stack scratchpad for DFS traversal
//!   and a fixed-size flat array (`AliasTable`) bounded by compile-time configuration limits.
//! * **L1 Cache Locality**: Variables are mapped directly to array indices, ensuring
//!   ultra-fast reads and sequential memory accesses.
//! * **One-Pass Linear Flattening**: Path compression is applied globally at the end of the
//!   extraction pass, rendering subsequent lookups strictly $O(1)$ and removing data-dependency
//!   bottlenecks from internal query loops.
//! * **Bitmask Cycle Protection**: The core `find` function implements a zero-overhead bitset
//!   within a raw CPU register (`settings::VariableMask`) to abort cyclic chains in a single instruction.
//!
//! # Strategic Inlining
//!
//! Critical helper functions (`new_alias_table`, `try_fetch_term`, `find`) are heavily inlined
//! (`#[inline(always)]`) to allow the compiler to fold memory layouts directly into register contexts,
//! avoiding call frame overhead inside critical processing pathways.

use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::{CompareOp, VariableId};
use crate::analysis::reachability::datalog::core::Term;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
use crate::analysis::reachability::datalog::settings;

/// A type alias representing the raw, flat table of variable aliases allocated on the stack.
///
/// This structure serves as the dense array backing for a zero-allocation, localized
/// Union-Find implementation. The fixed size is bounded by `settings::MAX_VARIABLES_PER_SCOPE`
/// to guarantee that the entire array fits within a single stack frame and maximizes
/// CPU L1 cache locality during resolution passes.
pub type AliasTable = [Term; settings::MAX_VARIABLES_PER_SCOPE];

/// Creates a default alias table where each variable maps directly to itself (the identity state).
///
/// This function initializes a stack-allocated [`AliasTable`] where every entry is populated
/// as a `Term::Variable` whose internal `VariableId` matches its corresponding array index.
/// This represents the initial state of a disjoint-set structure where each variable forms
/// its own singleton set.
///
/// # Performance Context
///
/// This function is decorated with `#[inline(always)]` to allow the compiler to construct
/// the array directly within the caller's stack allocation frame, completely eliminating
/// any function call overhead or redundant memory copying operations.
///
/// # Returns
///
/// A freshly initialized [`AliasTable`] ready for unification and aliasing passes.
#[inline(always)]
pub fn new_alias_table() -> AliasTable {
    std::array::from_fn(|i| Term::Variable(VariableId::from(i)))
}

/// Extracts variable aliases and canonical representatives from equality constraints in an expression.
///
/// This function traverses the given expression AST in a depth-first manner using a pre-allocated
/// scratchpad. It identifies equality comparisons (`= ?x ?y` or `= ?x c`) and populates a flat,
/// stack-allocated `AliasTable` using a high-performance, disjoint-set (Union-Find) approach.
///
/// Finally, it runs a linear flattening pass across the table so that every variable points
/// directly to its ultimate representative, ensuring future lookups are $O(1)$.
///
/// # Arguments
///
/// * `expr` - The root expression node ID to process.
/// * `scratchpad` - A mutable reference to a tracking buffer used to maintain the traversal stack and visited bitsets without allocation.
/// * `store` - A reference to the underlying expression database (`ExprStore`).
///
/// # Logic and Constraints
///
/// * **Negations**: Equalities nested directly inside a `Not` operator represent inequalities (`!=`) and are explicitly ignored.
/// * **Logical Contradictions**: Cases where two distinct constants are equated (directly or transitively via aliased variables) are handled silently. Leaving the entry unchanged guarantees unification will fail cleanly during Datalog saturation.
///
/// # Errors
///
/// Returns a [`DatalogError`] if a node cannot be fetched from the `ExprStore`, or if a variable identifier exceeds compile-time capacity constraints during term extraction.
///
/// # Returns
///
/// Returns an [`AliasTable`] where each index maps to its canonical `Term` representative.
pub(crate) fn compute_variable_aliasing(
    expr: ExprId,
    scratchpad: &mut DatalogScratchpad,
    store: &ExprStore,
) -> Result<AliasTable, DatalogError> {
    // ⚡ SINGLE SOURCE OF TRUTH FOR BUFFER CLEANUP
    scratchpad.prepare_visited(store.len());
    scratchpad.prepare_stack(expr);

    // 🚀 STACK ALLOCATION: Initialize each variable pointing to itself (identity)
    let mut alias_table: AliasTable = std::array::from_fn(|i| Term::Variable(VariableId::from(i)));

    while let Some(node_id) = scratchpad.stack.pop() {
        let idx = node_id.as_usize();
        if scratchpad.visited[idx] {
            continue;
        }
        scratchpad.visited[idx] = true;

        let node = store.fetch(node_id)?;
        let kind = node.kind();

        match kind {
            ExprKind::Not => {
                continue; // Equalities within a NOT are inequalities; safe to ignore.
            }

            ExprKind::Comparison(op) => {
                if *op == CompareOp::Equal {
                    let children = node.children();
                    if children.len() == 2 {
                        let t1 = try_fetch_term(children[0], store)?;
                        let t2 = try_fetch_term(children[1], store)?;

                        match (t1, t2) {
                            (Some(Term::Variable(v1)), Some(Term::Variable(v2))) => {
                                // Root resolution directly on the flat array ⚡
                                let r1 = find(v1, &alias_table);
                                let r2 = find(v2, &alias_table);

                                if r1 != r2 {
                                    match (r1, r2) {
                                        (Term::Variable(var1), Term::Variable(var2)) => {
                                            let v_max = var1.max(var2);
                                            let v_min = var1.min(var2);
                                            alias_table[v_max.as_usize()] = Term::Variable(v_min);
                                        }
                                        (Term::Variable(var), Term::Constant(c))
                                        | (Term::Constant(c), Term::Variable(var)) => {
                                            alias_table[var.as_usize()] = Term::Constant(c);
                                        }
                                        // Case (Constant, Constant) where c1 != c2:
                                        // Logical contradiction stemming from (= ?x ?y) when both variables hold distinct constants.
                                        // Handled silently here: keeping the table as-is triggers a clean unification failure in Datalog.
                                        _ => {}
                                    }
                                }
                            }
                            (Some(Term::Variable(v)), Some(Term::Constant(c)))
                            | (Some(Term::Constant(c)), Some(Term::Variable(v))) => {
                                // If the root is still a Variable, bind it to Constant c.
                                // If it is already a Constant, do nothing (silent handling of contradictions/dead branches).
                                if let Term::Variable(var) = find(v, &alias_table) {
                                    alias_table[var.as_usize()] = Term::Constant(c);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ => {
                for &child_id in node.children().iter().rev() {
                    scratchpad.stack.push(child_id);
                }
            }
        }
    }

    // ⚡ FINAL LINEAR FLATTENING: Path compression over the entire scope in a single pass
    for i in 0..settings::MAX_VARIABLES_PER_SCOPE {
        alias_table[i] = find(VariableId::from(i), &alias_table);
    }

    Ok(alias_table)
}

/// Extracts a term from an expression node and validates its bounds if it is a variable.
///
/// This helper function attempts to fetch and parse a [`Term`] from the given `ExprId`.
/// If the extracted term resolves to a logical variable, its identifier is strictly validated
/// against the compile-time scope capacity limits to prevent out-of-bounds operations downstream.
///
/// # Arguments
///
/// * `expr` - The unique identifier of the expression node to extract the term from.
/// * `store` - A reference to the `ExprStore` containing the node context.
///
/// # Constraints & Safety
///
/// Enforces that any extracted `VariableId` fits within the limits defined by
/// `settings::MAX_VARIABLES_PER_SCOPE`. This guarantees that the variable index can be safely
/// mapped to a single bit within a local CPU register bitmask context.
///
/// # Errors
///
/// Returns a [`DatalogError::VariableLimitExceeded`] if the resolved variable index equals
/// or exceeds the compile-time scope limit. It will also forward any underlying `DatalogError`
/// encountered while fetching the node from the `ExprStore`.
///
/// # Returns
///
/// * `Ok(Some(Term))` if a valid variable or constant term is extracted and passes validation.
/// * `Ok(None)` if the node is valid but does not represent a standalone term.
/// * `Err(DatalogError)` if fetching fails or the variable exceeds the scope limit.
#[inline(always)]
fn try_fetch_term(expr: ExprId, store: &ExprStore) -> Result<Option<Term>, DatalogError> {
    let term = as_term(expr, store)?;

    if let Some(Term::Variable(v)) = term {
        let limit = settings::MAX_VARIABLES_PER_SCOPE;
        if v.as_usize() >= limit {
            return Err(DatalogError::variable_limit_exceeded(v, limit));
        }
    }

    Ok(term)
}

/// Iteratively resolves a variable identifier to its canonical representative.
///
/// This function traverses the provided flat `AliasTable` to find the root element
/// or constant associated with a given `VariableId`. It acts as the core "Find"
/// operation within a stack-allocated Union-Find structure.
///
/// # Arguments
///
/// * `v` - The starting `VariableId` whose canonical representative needs to be found.
/// * `table` - A reference to the dense stack-allocated array representing the current variable aliases.
///
/// # Cycle Detection and Safety
///
/// To ensure absolute safety during traversal, the function employs a localized bitmask
/// (`settings::VariableMask`) acting as a zero-allocation visited set. This detects and
/// breaks cyclic aliases within a single CPU instruction context, completely avoiding infinite loops.
///
/// # Constraints & Safety
///
/// If the resolved variable's index equals or exceeds `settings::MAX_VARIABLES_PER_SCOPE`,
/// traversal is aborted immediately, returning the current variable to prevent bit-shift
/// overflows or out-of-bounds memory accesses.
///
/// # Returns
///
/// The final canonical [`Term`], which is either the root `Term::Variable` of the disjoint set
/// or a `Term::Constant` if the variable chain has been bound to a concrete object.
#[inline(always)]
pub(crate) fn find(v: VariableId, table: &AliasTable) -> Term {
    let idx = v.as_usize();
    // BOUNDS SAFETY: Prevents bit-shift overflow if the ID >= settings::MAX_VARIABLES_PER_SCOPE
    if idx >= settings::MAX_VARIABLES_PER_SCOPE {
        return Term::Variable(v);
    }

    let mut curr_term = Term::Variable(v);
    let mut visited_mask: settings::VariableMask = 0;

    while let Term::Variable(curr_var) = curr_term {
        let idx = curr_var.as_usize();

        // Local loop safety via binary register bitmask (1 CPU cycle)
        let bit = (1 as settings::VariableMask) << idx;
        if (visited_mask & bit) != 0 {
            break;
        }
        visited_mask |= bit;

        let next_term = &table[idx];
        if let Term::Variable(next_var) = next_term {
            if *next_var == curr_var {
                break;
            }
        }
        curr_term = *next_term;
    }

    curr_term
}

/// Converts an expression node identifier into a Datalog term representation.
///
/// This local helper function inspects the given `ExprId` within the provided `ExprStore`
/// to determine if it maps to a valid Datalog component (either a logical variable or
/// a concrete object constant).
///
/// # Arguments
///
/// * `expr` - The unique identifier of the expression node to be inspected.
/// * `store` - A reference to the storage context (`ExprStore`) containing the node details.
///
/// # Logic and Behavioral Variants
///
/// The function fetches the expression node and matches against its internal `ExprKind`:
/// * `ExprKind::Variable(v_id)` $\rightarrow$ Returns `Some(Term::Variable(*v_id))`
/// * `ExprKind::Object(obj_id)` $\rightarrow$ Returns `Some(Term::Constant(*obj_id))`
/// * Any other kind (e.g., operators, compound expressions) $\rightarrow$ Returns `None`
///
/// # Errors
///
/// Returns a [`DatalogError`] if the `store` fails to look up or fetch the node
/// associated with the provided `expr` ID (e.g., due to an invalid or out-of-bounds identifier).
///
/// # Returns
///
/// * `Ok(Some(Term))` if the expression is successfully identified as a variable or object.
/// * `Ok(None)` if the expression is valid but does not represent a standalone term.
/// * `Err(DatalogError)` if a storage resolution failure occurs.
fn as_term(expr: ExprId, store: &ExprStore) -> Result<Option<Term>, DatalogError> {
    let n = store.fetch(expr)?;

    Ok(match n.kind() {
        ExprKind::Variable(v_id) => Some(Term::Variable(*v_id)),
        ExprKind::Object(obj_id) => Some(Term::Constant(*obj_id)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprStore};
    use crate::aiplan4rust::support::lang::{CompareOp, ObjectId, VariableId};
    use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;

    #[test]
    fn test_new_alias_table_identity() {
        let table = new_alias_table();
        for i in 0..settings::MAX_VARIABLES_PER_SCOPE {
            let var = VariableId::from(i);
            assert_eq!(find(var, &table), Term::Variable(var));
        }
    }

    #[test]
    fn test_find_with_direct_aliasing() {
        let mut table = new_alias_table();
        // Map ?1 to ?0
        table[1] = Term::Variable(VariableId::from(0));

        assert_eq!(
            find(VariableId::from(1), &table),
            Term::Variable(VariableId::from(0))
        );
    }

    #[test]
    fn test_find_with_constant_binding() {
        let mut table = new_alias_table();
        let obj = ObjectId::from(42);
        // Map ?2 to Constant(42)
        table[2] = Term::Constant(obj);

        assert_eq!(find(VariableId::from(2), &table), Term::Constant(obj));
    }

    #[test]
    fn test_find_cycle_protection() {
        let mut table = new_alias_table();
        // Create a synthetic cycle: ?0 -> ?1, ?1 -> ?0
        table[0] = Term::Variable(VariableId::from(1));
        table[1] = Term::Variable(VariableId::from(0));

        // The bitmask cycle protection should break the loop and return safely
        let result = find(VariableId::from(0), &table);
        assert!(matches!(result, Term::Variable(_)));
    }

    #[test]
    fn test_compute_variable_aliasing_simple_equality() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));
        let v1 = builder.variable(VariableId::from(1));

        // (= ?0 ?1) via ExprBuilder
        let eq_expr = builder.comparison(CompareOp::Equal, v0, v1);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(eq_expr, &mut scratchpad, &store).unwrap();

        assert_eq!(
            find(VariableId::from(1), &table),
            Term::Variable(VariableId::from(0))
        );
    }

    #[test]
    fn test_as_term_extraction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_expr = builder.variable(VariableId::from(5));
        let obj_expr = builder.object(ObjectId::from(10));
        let op_expr = builder.and(&vec![]);

        assert_eq!(
            as_term(var_expr, &store).unwrap(),
            Some(Term::Variable(VariableId::from(5)))
        );
        assert_eq!(
            as_term(obj_expr, &store).unwrap(),
            Some(Term::Constant(ObjectId::from(10)))
        );
        assert_eq!(as_term(op_expr, &store).unwrap(), None);
    }

    #[test]
    fn test_compute_variable_aliasing_transitive() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));

        // (= ?0 ?1) AND (= ?1 ?2)
        let eq1 = builder.comparison(CompareOp::Equal, v0, v1);
        let eq2 = builder.comparison(CompareOp::Equal, v1, v2);
        let and_expr = builder.and(&vec![eq1, eq2]);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(and_expr, &mut scratchpad, &store).unwrap();

        // Transitive flattening should map ?2 -> ?0
        assert_eq!(
            find(VariableId::from(2), &table),
            Term::Variable(VariableId::from(0))
        );
    }

    #[test]
    fn test_compute_variable_aliasing_constant_binding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));
        let c10 = builder.object(ObjectId::from(10));

        // (= ?0 10)
        let eq_expr = builder.comparison(CompareOp::Equal, v0, c10);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(eq_expr, &mut scratchpad, &store).unwrap();

        assert_eq!(
            find(VariableId::from(0), &table),
            Term::Constant(ObjectId::from(10))
        );
    }

    #[test]
    fn test_compute_variable_aliasing_ignores_not() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));
        let v1 = builder.variable(VariableId::from(1));

        // (NOT (= ?0 ?1)) -> Should be ignored by alias extraction
        let eq = builder.comparison(CompareOp::Equal, v0, v1);
        let not_expr = builder.not(eq);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(not_expr, &mut scratchpad, &store).unwrap();

        // Should remain identity mapping
        assert_eq!(
            find(VariableId::from(1), &table),
            Term::Variable(VariableId::from(1))
        );
    }

    #[test]
    fn test_try_fetch_term_variable_limit_exceeded() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create a variable ID that exceeds MAX_VARIABLES_PER_SCOPE
        let invalid_var_id = VariableId::from(settings::MAX_VARIABLES_PER_SCOPE + 5);
        let invalid_var_expr = builder.variable(invalid_var_id);
        let valid_var_expr = builder.variable(VariableId::from(0));

        // Wrap it in an equality comparison so try_fetch_term gets executed
        let eq_expr = builder.comparison(CompareOp::Equal, invalid_var_expr, valid_var_expr);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let result = compute_variable_aliasing(eq_expr, &mut scratchpad, &store);

        assert!(result.is_err());
    }

    #[test]
    fn test_compute_variable_aliasing_symmetric_constant_binding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let c10 = builder.object(ObjectId::from(10));
        let v0 = builder.variable(VariableId::from(0));

        // (= 10 ?0) -> Constant on the left side
        let eq_expr = builder.comparison(CompareOp::Equal, c10, v0);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(eq_expr, &mut scratchpad, &store).unwrap();

        assert_eq!(
            find(VariableId::from(0), &table),
            Term::Constant(ObjectId::from(10))
        );
    }

    #[test]
    fn test_compute_variable_aliasing_self_equality() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));

        // (= ?0 ?0)
        let eq_expr = builder.comparison(CompareOp::Equal, v0, v0);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(eq_expr, &mut scratchpad, &store).unwrap();

        assert_eq!(
            find(VariableId::from(0), &table),
            Term::Variable(VariableId::from(0))
        );
    }

    #[test]
    fn test_compute_variable_aliasing_conflicting_constants() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let v0 = builder.variable(VariableId::from(0));
        let c10 = builder.object(ObjectId::from(10));
        let c20 = builder.object(ObjectId::from(20));

        // (= ?0 10) AND (= ?0 20) -> Contradiction handled silently
        let eq1 = builder.comparison(CompareOp::Equal, v0, c10);
        let eq2 = builder.comparison(CompareOp::Equal, v0, c20);
        let and_expr = builder.and(&vec![eq1, eq2]);

        let mut scratchpad = DatalogScratchpad::with_capacity(64);
        let table = compute_variable_aliasing(and_expr, &mut scratchpad, &store).unwrap();

        // The first binding should be preserved while the second is ignored silently
        assert_eq!(
            find(VariableId::from(0), &table),
            Term::Constant(ObjectId::from(10))
        );
    }
}
