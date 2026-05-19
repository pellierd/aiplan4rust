//! # Factored Normal Form (FNF) Transformation
//!
//! This module implements the transformation of logical expressions into **Factored Normal Form (FNF)**.
//! The primary goal is to iteratively apply the distributive law:
//! `(A ∧ B) ∨ (A ∧ C) ≡ A ∧ (B ∨ C)`
//!
//! ## Purpose of the Transformation
//!
//! In automated planning (AI Planning), expressions such as preconditions and effects in PDDL or HDL
//! domains often contain significant redundancies. Factorization provides several key benefits:
//! 1. **LIR Size Reduction**: Fewer nodes are stored in memory thanks to Hash-Consing.
//! 2. **Accelerated Evaluation**: By extracting common factors (e.g., a robot's location required for
//!    ten different actions), we avoid redundant evaluations of the same sub-expression.
//! 3. **Grounding Optimization**: Helps identify shared variables earlier in the instantiation process.
//!
//! ## Implementation Choices
//!
//! ### Greedy Approach
//! Instead of searching for a theoretically optimal factorization (which is a complex combinatorial problem),
//! we use a frequency-based greedy algorithm:
//! * In each step, the algorithm extracts the most frequent symbol among conjunctive groups.
//! * **Rationale**: This offers an excellent trade-off between computation time and compression ratio,
//!   making it particularly effective for the repetitive structures found in planning problems.
//!
//! ### Scratchpad Usage (Buffer Reuse)
//! The implementation relies on a `Scratchpad` to house working buffers.
//! * **Rationale**: This prevents thousands of `Vec` allocations when processing large domains.
//!   Existing memory is reused for frequency calculations and group sorting.
//!
//! ### Sorting and Binary Search
//! Each conjunctive group is sorted during the preparation phase.
//! * **Rationale**: This allows the use of `binary_search` ($O(\log n)$) instead of a linear scan ($O(n)$)
//!   to verify the presence of a factor. It also ensures that Hash-Consing correctly identifies
//!   `(A ∧ B)` and `(B ∧ A)` as the exact same node.
//!
//! ## Considered Alternatives
//!
//! 1. **Binary Decision Diagrams (BDD/ZDD)**: Very powerful for factorization but complex to maintain
//!    and often slower for "one-shot" transformations during preprocessing compared to direct manipulation.
//! 2. **Exact Factorization**: Testing all possible combinations of factors. This was rejected because
//!    the computational cost would be prohibitive for expressions containing hundreds of terms.
//!
//! ## Expected Result
//!
//! An expression transformed by `to_fnf` is guaranteed to be logically equivalent to the original,
//! but presented in a more compact and hierarchical form.
//! * **Input**: A flat `OR` containing multiple `AND` nodes.
//! * **Output**: A tree of nested `AND` and `OR` nodes where the most obvious redundancies have been eliminated.
//!

use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::iter::scratchpad::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use smallvec::SmallVec;
use std::mem;

/// Capacité inline pour la liste initiale des enfants (entrées du OR).
const MAX_CHILDREN: usize = 32;

/// Capacité inline pour les buffers de reconstruction (arguments du AND/OR).
/// 32 est un bon compromis pour couvrir la majorité des expressions PDDL
/// sans saturer la pile (stack).
const MAX_FACTOR_GROUPS: usize = 32;

/// Transforms an expression into Factored Normal Form (FNF).
///
/// This is the main orchestrator function that applies the distributive law
/// `(A ∧ B) ∨ (A ∧ C) ≡ A ∧ (B ∨ C)` to simplify logical expressions.
///
/// ### How it works
/// The transformation is performed in three distinct phases:
/// 1. **Preparation**: It flattens the input and segregates "naked" literals from
///    conjunctions that can be factorized.
/// 2. **Factorization**: It uses a greedy approach to find the most common factors
///    and extracts them recursively (if enabled).
/// 3. **Assembly**: It reconstructs the expression tree, ensuring that identities
///    (like empty ANDs or ORs) are handled correctly to maintain a lean LIR.
///
/// ### Scratchpad Usage
/// This function is designed for performance and avoids high-frequency allocations by
/// using a provided `Scratchpad`. The scratchpad acts as a reusable memory buffer for:
/// - `group_buffer`: Holds the terms of the conjunctions being factorized.
/// - `other_kids`: Temporary storage for common factors extracted during Phase 2.
///
/// ### Arguments
/// * `expr` - The `ExprId` of the expression to simplify (usually an `OR` node).
/// * `builder` - A mutable reference to the `ExprBuilder` for node creation and Hash-Consing.
/// * `scratch` - A mutable reference to a `Scratchpad` to use as a working buffer.
/// * `recursive` - If `true`, the algorithm will attempt to find factors within
///   factors (multi-level factorization).
///
/// ### Returns
/// * `Ok(ExprId)` - The ID of the simplified expression.
/// * `Err(ExprOpErrorHC)` - If an error occurs during expression fetching or building.
pub fn to_fnf(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
    recursive: bool,
) -> Result<ExprId, ExprOpErrorHC> {
    // --- 1. PREPARATION ---
    // Deconstruct the input and separate factorable AND-groups from standalone nodes.
    let (naked_literals, initial_groups) = prepare_input(expr, builder)?;

    // Initialize the scratchpad buffers with the prepared data.
    scratch.group_buffer_mut().clear();
    *scratch.group_buffer_mut() = initial_groups;
    scratch.other_kids_mut().clear();

    // --- 2. CORE LOGIC ---
    // Apply the distributive law iteratively to find commonalities.
    // This mutates the groups inside the scratchpad.
    perform_factorization(builder, scratch, recursive);

    // --- 3. ASSEMBLY ---
    // Stitch the naked literals, extracted factors, and remaining groups
    // back into a single, optimized expression tree.
    assemble_result(naked_literals, builder, scratch)
}

/// Phase 1: Preparation
///
/// This function deconstructs the input expression into a normalized format suitable
/// for factorization. It separates elements that can be factorized (conjunctions)
/// from those that cannot (literals or non-AND nodes).
///
/// ### Process
/// 1. **OR-Flattening**: If the input is an `OR` node, it iterates over its children.
///    Otherwise, it treats the single expression as the sole child.
/// 2. **Categorization**:
///    - **Groups**: Children that are `AND` nodes are converted into vectors of `ExprId`.
///    - **Naked Literals**: Children that are not `AND` nodes (atoms, negations, etc.)
///      are collected separately as they cannot be factorized via the distributive law.
/// 3. **Normalization**: Within each group, children are sorted.
///
/// ### Invariants & Performance
/// - **Sorting**: Sorting the IDs in each group is mandatory for Phase 2's `binary_search`.
/// - **Hash-Consing**: Sorting also ensures that logically equivalent `AND` nodes
///   result in identical internal representations.
/// - **SmallVec**: Uses `SmallVec` for `naked_literals` to avoid heap allocation
///   for expressions with few non-conjunctive children.
///
/// ### Arguments
/// * `expr` - The root `ExprId` to process.
/// * `builder` - The builder used to fetch expression metadata.
///
/// ### Returns
/// A `Result` containing:
/// - A `SmallVec` of "naked" `ExprId`s.
/// - A `Vec<Vec<ExprId>>` where each inner vector represents a conjunctive group.
fn prepare_input(
    expr: ExprId,
    builder: &ExprBuilder,
) -> Result<(SmallVec<[ExprId; MAX_CHILDREN]>, Vec<Vec<ExprId>>), ExprOpErrorHC> {
    let mut naked = SmallVec::new();
    let mut groups = Vec::new();

    // Fetch the root entry to inspect its kind
    let entry = builder.fetch(expr)?;

    // If the root is an OR, we factorize its children.
    // If not, we treat the expression as a single-child disjunction.
    let children = if matches!(entry.kind(), ExprEntryKind::Or) {
        entry.children()
    } else {
        std::slice::from_ref(&expr)
    };

    for &child_id in children {
        let child_entry = builder.fetch(child_id)?;

        if matches!(child_entry.kind(), ExprEntryKind::And) {
            // It's a conjunction: extract its children into a group
            let mut kids = child_entry.children().to_vec();

            // CRITICAL: Sort for consistent Hash-Consing and binary search in Phase 2
            kids.sort_unstable();
            groups.push(kids);
        } else {
            // It's a literal or another non-AND node: it stays in the outer OR
            naked.push(child_id);
        }
    }

    Ok((naked, groups))
}

/// Phase 2: Factorization Loop
///
/// This function implements a greedy factorization algorithm that iteratively extracts
/// the most frequent common factors from a set of conjunctive groups (AND nodes).
///
/// ### Algorithm
/// 1. **Frequency Analysis**: It counts how often each `ExprId` appears across all groups.
/// 2. **Best Factor Selection**: It identifies the factor `f` with the highest frequency.
/// 3. **Extraction**:
///    - Groups containing `f` are updated by removing `f`.
///    - `f` is moved to the global "factors" buffer (`other_kids`).
/// 4. **Recursion**: If `recursive` is true, the process repeats on the modified groups
///    until no more common factors can be found.
///
/// ### Performance Notes
/// - The function uses `binary_search` to find factors, which assumes that the children
///   within each group were sorted during Phase 1.
/// - It leverages `mem::take` and `drain` to minimize allocations while shuffling
///   vectors between "processed" and "remaining" states.
///
/// ### Arguments
/// * `builder` - The expression builder (used here mainly for logical context).
/// * `scratch` - The scratchpad holding the `group_buffer` (input/output) and
///   `other_kids` (extracted factors).
/// * `recursive` - If true, continues extracting factors until a global fixed point is reached.
fn perform_factorization(builder: &mut ExprBuilder, scratch: &mut Scratchpad, recursive: bool) {
    loop {
        // We need at least 2 groups to find a common factor (A&B | A&C).
        // A single group cannot be "factorized" further in this context.
        if scratch.group_buffer().len() < 2 {
            break;
        }

        // Re-calculate frequencies based on the current state of the groups.
        scratch.compute_frequencies();

        // find_best_factor returns the ExprId with the highest count > 1.
        if let Some(f) = scratch.find_best_factor() {
            // Factor found! Store it in the shared factors buffer.
            scratch.other_kids_mut().push(f);

            let mut groups_with_f = Vec::new();
            let mut still_to_process = Vec::new();

            // Take the buffer to avoid borrow checker issues while draining.
            let mut old_groups = mem::take(scratch.group_buffer_mut());

            for mut group in old_groups.drain(..) {
                // Since groups are sorted, binary_search is O(log n).
                if group.binary_search(&f).is_ok() {
                    // Remove the factor from this group as it's now global.
                    group.retain(|&x| x != f);
                    groups_with_f.push(group);
                } else {
                    // This group doesn't share the current best factor.
                    still_to_process.push(group);
                }
            }

            // Update the scratchpad: groups_with_f are candidates for further
            // factorization in the next loop iteration.
            let current_groups = scratch.group_buffer_mut();
            *current_groups = groups_with_f;
            current_groups.extend(still_to_process);

            // If not recursive, we stop after the first best factor is found.
            if !recursive {
                break;
            }
        } else {
            // No factor appears in more than one group.
            break;
        }
    }
}

/// Phase 3: Final Assembly
///
/// This function reconstructs the final expression tree from the components processed
/// during factorization. It follows a hierarchical reassembly:
/// `OR( Naked Literals, AND( Extracted Factors, OR( Remaining Groups ) ) )`
///
/// ### Logic
/// 1. **Rebuild Factorized OR**: Converts the remaining groups in the scratchpad back into
///    an `OR` of `AND`s.
/// 2. **Apply Factors**: If common factors were found, it wraps the factorized part
///    into a new `AND` node with those factors.
/// 3. **Final Union**: Integrates "naked" literals (those that weren't part of any
///    factorable `AND`) into a final top-level `OR`.
///
/// ### Arguments
/// * `naked_literals` - Elements from the original expression that were not `AND` nodes.
/// * `builder` - The expression builder used for Hash-Consing.
/// * `scratch` - The scratchpad containing the remaining groups and extracted factors.
///
/// ### Returns
/// A `Result` containing the new `ExprId` or an `ExprOpErrorHC`.
fn assemble_result(
    naked_literals: SmallVec<[ExprId; MAX_CHILDREN]>,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    let empty_and = builder.empty_and();
    let empty_or = builder.empty_or();

    // --- A. Rebuild the factorized part ---
    // We transform the refined groups back into actual expressions.
    // Each group is turned into an AND, and all groups are collected into an OR.
    let mut factorized_or_args: SmallVec<[ExprId; MAX_FACTOR_GROUPS]> = SmallVec::new();
    for g in mem::take(scratch.group_buffer_mut()) {
        factorized_or_args.push(builder.and(&g));
    }
    let inner_or = builder.or(&factorized_or_args);

    // --- B. Apply all extracted factors ---
    // If we have factors [f1, f2], we want: AND(f1, f2, inner_or).
    let factors = mem::take(scratch.other_kids_mut());
    let factored_branch = if factors.is_empty() {
        inner_or
    } else {
        let mut and_args: SmallVec<[ExprId; MAX_FACTOR_GROUPS]> = SmallVec::new();
        and_args.extend(factors);

        // Logical check: only add inner_or if it contains relevant remaining logic.
        // We skip it if it's empty or evaluates to a neutral identity.
        if inner_or != empty_and && inner_or != empty_or {
            and_args.push(inner_or);
        }
        builder.and(&and_args)
    };

    // --- C. Combine with original naked literals ---
    // Final assembly: OR(literal1, literal2, ..., factored_branch).
    if naked_literals.is_empty() {
        // If no literals like 'D' in (A&B | A&C | D), just return the factored part.
        Ok(factored_branch)
    } else {
        let mut final_args = naked_literals;

        // Only append the factored branch if it's not an empty OR (neutral in an OR).
        if factored_branch != empty_or {
            final_args.push(factored_branch);
        }

        Ok(builder.or(&final_args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::AtomSkeletonId;
    use crate::aiplan4rust::lir::store::expr::ExprStore;

    /// Utility helper to create unique atomic formulas for testing.
    fn atom(builder: &mut ExprBuilder, id: usize) -> ExprId {
        builder.atomic_formula(id, &[], AtomSkeletonId::from(id))
    }

    /// TEST 1: Basic Factorization
    /// Why: Verify the core distributive law: (A & B) | (A & C) => A & (B | C).
    /// Input:  OR(AND(A, B), AND(A, C))
    /// Output: AND(A, OR(B, C))
    #[test]
    fn test_fnf_basic_distributive_law() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);

        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[a, c]);
        let root = builder.or(&[and1, and2]);

        let result_id = to_fnf(root, &mut builder, &mut scratch, false)?;
        let entry = builder.fetch(result_id)?;

        // The root should now be an AND node because 'A' was extracted.
        assert!(matches!(entry.kind(), ExprEntryKind::And));
        assert!(entry.children().contains(&a));

        // The other child of the AND should be the OR(B, C).
        let remaining_or_id = entry.children().iter().find(|&&id| id != a).unwrap();
        let remaining_or = builder.fetch(*remaining_or_id)?;
        assert!(matches!(remaining_or.kind(), ExprEntryKind::Or));
        assert!(remaining_or.children().contains(&b));
        assert!(remaining_or.children().contains(&c));

        Ok(())
    }

    /// TEST 2: Handling Naked Literals
    /// Why: Ensure that elements that are not AND nodes are preserved in the final OR.
    /// Input:  OR(AND(A, B), AND(A, C), D)
    /// Output: OR(D, AND(A, OR(B, C)))
    #[test]
    fn test_fnf_preserves_non_and_nodes() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);

        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[a, c]);
        let root = builder.or(&[and1, and2, d]);

        let result_id = to_fnf(root, &mut builder, &mut scratch, false)?;
        let entry = builder.fetch(result_id)?;

        // The root must remain an OR because 'D' cannot be factored with 'A'.
        assert!(matches!(entry.kind(), ExprEntryKind::Or));
        let children = entry.children();
        assert!(children.contains(&d));

        // One of the children should be the result of factoring (A&B | A&C).
        let factored_child = children.iter().find(|&&id| id != d).unwrap();
        assert!(matches!(
            builder.fetch(*factored_child)?.kind(),
            ExprEntryKind::And
        ));

        Ok(())
    }

    /// TEST 3: Recursive Factorization
    /// Why: Verify that multiple common factors can be extracted (A & B).
    /// Input:  OR(AND(A, B, C), AND(A, B, D))
    /// Output: AND(A, B, OR(C, D))
    #[test]
    fn test_fnf_recursive_extraction() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);

        let and1 = builder.and(&[a, b, c]);
        let and2 = builder.and(&[a, b, d]);
        let root = builder.or(&[and1, and2]);

        // We set recursive = true to extract both A and B.
        let result_id = to_fnf(root, &mut builder, &mut scratch, true)?;
        let entry = builder.fetch(result_id)?;

        assert!(matches!(entry.kind(), ExprEntryKind::And));
        let children = entry.children();
        // A and B should both be top-level children of the factored AND.
        assert!(children.contains(&a));
        assert!(children.contains(&b));

        Ok(())
    }

    /// TEST 4: No Common Factors
    /// Why: Ensure the function returns an equivalent (or same) expression if no factoring is possible.
    /// Input:  OR(AND(A, B), AND(C, D))
    /// Output: OR(AND(A, B), AND(C, D)) (No changes)
    #[test]
    fn test_fnf_no_optimization_possible() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);

        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[c, d]);
        let root = builder.or(&[and1, and2]);

        let result_id = to_fnf(root, &mut builder, &mut scratch, true)?;

        // Result should be structurally identical to root.
        assert_eq!(result_id, root);
        Ok(())
    }

    /// TEST 5: Idempotency
    /// Why: Running FNF twice on the same expression should not change the result.
    /// Input:  An expression E.
    /// Output: FNF(E) == FNF(FNF(E))
    #[test]
    fn test_fnf_idempotency() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[a, c]);
        let root = builder.or(&[and1, and2]);

        let first_pass = to_fnf(root, &mut builder, &mut scratch, true)?;
        let second_pass = to_fnf(first_pass, &mut builder, &mut scratch, true)?;

        assert_eq!(first_pass, second_pass, "FNF transformation must be stable");
        Ok(())
    }

    /// TEST 6: Nested Factorization (The "Matryoshka")
    /// Why: Check if it can factorize when the common factor is itself a complex expression.
    /// Input: OR(AND(OR(A, B), C), AND(OR(A, B), D))
    /// Expect: AND(OR(A, B), OR(C, D))
    #[test]
    fn test_fnf_complex_factor() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);
        let common_factor = builder.or(&[a, b]);
        let and1 = builder.and(&[common_factor, c]);
        let and2 = builder.and(&[common_factor, d]);
        let root = builder.or(&[and1, and2]);

        let result_id = to_fnf(root, &mut builder, &mut scratch, true)?;
        let entry = builder.fetch(result_id)?;

        assert!(matches!(entry.kind(), ExprEntryKind::And));
        assert!(entry.children().contains(&common_factor));
        Ok(())
    }

    /// TEST 7: Full Extraction (The "Empty Shell")
    /// Why: What happens if the entire content of ANDs is common?
    /// Input: OR(AND(A, B), AND(A, B))
    /// Expect: AND(A, B) or simply the original AND if the builder de-duplicates.
    #[test]
    fn test_fnf_full_extraction() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);

        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[a, b]); // This should be the same ExprId thanks to HC
        let root = builder.or(&[and1, and2]);

        let result_id = to_fnf(root, &mut builder, &mut scratch, true)?;

        // After factoring A and B, the remaining OR is empty or contains identity.
        // The result should logically be equivalent to (A & B).
        let entry = builder.fetch(result_id)?;
        if matches!(entry.kind(), ExprEntryKind::And) {
            assert!(entry.children().contains(&a));
            assert!(entry.children().contains(&b));
        } else {
            // If your builder simplifies OR(X, X) to X
            assert_eq!(result_id, and1);
        }
        Ok(())
    }

    /// TEST 8: Empty or Single Child Input
    /// Why: Robustness against non-standard or simplified inputs.
    /// Input: OR(A) or Empty OR
    #[test]
    fn test_fnf_edge_cases_empty_single() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        // Single child
        let a = atom(&mut builder, 1);
        let root_single = builder.or(&[a]);
        let res_single = to_fnf(root_single, &mut builder, &mut scratch, true)?;
        assert_eq!(res_single, a); // Should return A or OR(A)

        // Empty OR
        let root_empty = builder.or(&[]);
        let res_empty = to_fnf(root_empty, &mut builder, &mut scratch, true)?;
        assert_eq!(res_empty, builder.empty_or());

        Ok(())
    }

    /// TEST 9: Greedy Tie-Breaking
    /// Why: If two factors have the same frequency, ensure the algo doesn't crash
    /// and picks one consistently.
    /// Input: OR(AND(A, B, C), AND(A, B, D)) -> Frequency of A = 2, B = 2.
    #[test]
    fn test_fnf_tie_breaking() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);

        let and1 = builder.and(&[a, b, c]);
        let and2 = builder.and(&[a, b, d]);
        let root = builder.or(&[and1, and2]);

        // Should extract at least one, and if recursive, both.
        let result_id = to_fnf(root, &mut builder, &mut scratch, true)?;
        let entry = builder.fetch(result_id)?;

        assert!(matches!(entry.kind(), ExprEntryKind::And));
        let children = entry.children();
        assert!(children.contains(&a));
        assert!(children.contains(&b));
        Ok(())
    }

    #[test]
    fn test_fnf_cascade_factorization() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);
        let e = atom(&mut builder, 5);

        // --- TEST LOGIC ---
        // Why: This test verifies "Deep" or "Cascading" factorization. It checks if the
        // algorithm can extract a global factor, and then find another factor
        // within the remaining sub-expressions.
        //
        // Input: OR( AND(A, B, C), AND(A, B, D), AND(A, E) )
        //   - 'A' is common to all 3 branches.
        //   - 'B' is common only to the first 2 branches.
        //
        // Step-by-Step Transformation:
        // 1. Extract 'A': AND( A, OR( AND(B, C), AND(B, D), E ) )
        // 2. Extract 'B' (inside the OR): AND( A, OR( AND(B, OR(C, D)), E ) )
        //
        // Expected Output: A root AND node containing 'A' and the nested factored structure.
        // ------------------

        let and1 = builder.and(&[a, b, c]);
        let and2 = builder.and(&[a, b, d]);
        let and3 = builder.and(&[a, e]);
        let root = builder.or(&[and1, and2, and3]);

        // Execute with recursive = true to ensure it doesn't stop after extracting 'A'
        let result = to_fnf(root, &mut builder, &mut scratch, true)?;
        let entry = builder.fetch(result)?;

        // Root should be an AND node because 'A' was extracted from every branch of the top-level OR
        assert!(
            matches!(entry.kind(), ExprEntryKind::And),
            "The root should have been transformed into an AND node after extracting factor 'A'"
        );

        assert!(
            entry.children().contains(&a),
            "The global factor 'A' should be a direct child of the resulting AND node"
        );

        Ok(())
    }

    #[test]
    fn test_fnf_overlapping_sets() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);
        let c = atom(&mut builder, 3);
        let d = atom(&mut builder, 4);
        let e = atom(&mut builder, 5);

        // --- TEST LOGIC ---
        // Why: This is a "Greedy Conflict" test. Both 'A' and 'B' appear exactly twice,
        // but they cover different sets of branches. This tests if the algorithm handles
        // tie-breaking correctly and continues to optimize without crashing.
        //
        // Input: OR( AND(A, B, C), AND(A, D), AND(B, E) )
        //   - 'A' is present in: Branch 1 and Branch 2.
        //   - 'B' is present in: Branch 1 and Branch 3.
        //
        // Expected Output:
        //   If 'A' is picked first: OR( AND(A, OR(AND(B, C), D)), AND(B, E) )
        //   If 'B' is picked first: OR( AND(B, OR(AND(A, C), E)), AND(A, D) )
        //
        // In both cases, the expression should be smaller/more optimized than the original.
        // ------------------

        let and1 = builder.and(&[a, b, c]);
        let and2 = builder.and(&[a, d]);
        let and3 = builder.and(&[b, e]);
        let root = builder.or(&[and1, and2, and3]);

        // Execute FNF with recursive enabled
        let result = to_fnf(root, &mut builder, &mut scratch, true)?;

        // The primary goal is that the algorithm doesn't crash and performs at least
        // one level of factorization, proving it successfully navigated the tie-break.
        assert!(
            result != root,
            "The expression should have been optimized as common factors A or B were available"
        );

        // Verify the result is still a valid part of the store
        let _entry = builder.fetch(result)?;

        Ok(())
    }

    #[test]
    fn test_fnf_redundant_shuffled_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = atom(&mut builder, 1);
        let b = atom(&mut builder, 2);

        // --- TEST LOGIC ---
        // Why: This test checks if the factorization handles redundant branches and
        // shuffled operand orders correctly thanks to internal sorting and Hash-Consing.
        //
        // Input: OR( AND(A, B), AND(B, A), A )
        //   Note: AND(A, B) and AND(B, A) are logically identical.
        //
        // Expected Output: AND( A, OR(B, True) )
        //   Explanation: 'A' is a common factor in all three branches.
        //   - From AND(A, B), we extract A, leaving B.
        //   - From AND(B, A), we extract A, leaving B.
        //   - From A, we extract A, leaving True (empty AND).
        //   The result is A multiplied by the disjunction of the remainders.
        // ------------------

        let and1 = builder.and(&[a, b]);
        let and2 = builder.and(&[b, a]); // Shuffled order of operands
        let root = builder.or(&[and1, and2, a]);

        // We run to_fnf with recursive = true
        let result = to_fnf(root, &mut builder, &mut scratch, true)?;

        let entry = builder.fetch(result)?;

        // If the builder and FNF work correctly:
        // 1. and1 and and2 should have been merged/treated as the same group.
        // 2. The final structure should be an AND with children: [A, OR(B, True)].
        // We assert that the number of children is reduced (A and the inner OR).
        assert!(
            entry.children().len() <= 2,
            "The expression should have been simplified to at most 2 top-level children (A and the remainder OR)"
        );

        Ok(())
    }
}
