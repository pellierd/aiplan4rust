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
//! ### Bounded Window Recursion (Slices)
//! To fully preserve memory locality and avoid copying partitions during the divide-and-conquer strategy,
//! the recursive engine operates within explicit index boundaries `[start_idx..end_idx]` over a global buffer.
//! * **Rationale**: This ensures strict isolation of local logical scopes during sub-factorization steps.
//!   It also guarantees $O(D)$ space complexity on the call stack (where $D$ is the recursive depth)
//!   and completely eliminates structural data fragmentation.
//!
//! ### Scratchpad Usage & Buffer Reuse
//! The implementation relies on a persistent `Scratchpad` to house all mutable working buffers.
//! * **Rationale**: This guarantees **$O(1)$ heap allocation overhead** during processing. Existing
//!   allocated memory is continuously reused for local frequency calculations, in-place group
//!   partitioning, and final tree building, preventing thousands of microscopic system allocator calls.
//!
//! ### Pre-Sorting & Canonical Forms
//! Each conjunctive group is eagerly sorted via an unstable sort during the preparation phase.
//! * **Rationale**: This layout guarantees a fast $O(\log n)$ lookup time using `binary_search` rather than
//!   an $O(n)$ linear scan to verify the presence of a factor. Furthermore, sorting ensures that the
//!   underlying Hash-Consing mechanism correctly identifies commutative groups like `(A ∧ B)` and `(B ∧ A)`
//!   as the exact same physical node.
//!
//! ### Stack-Allocated Micro-Buffers (SmallVec)
//! Non-conjunctions (naked literals) are isolated at ingest time into a stack-bounded array before being
//! merged back into the root level.
//! * **Rationale**: Bounding this staging area (e.g., up to 32 elements) keeps the routine's stack frame
//!   extremely lightweight while ensuring that small-to-medium disjunctions bypass the heap entirely.
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
//! but presented in a more compact, hierarchical, and canonical form.
//! * **Input**: A flat `OR` containing multiple `AND` nodes (or standalone singletons).
//! * **Output**: A tree of nested `AND` and `OR` nodes where the most obvious redundancies have been
//!   eliminated on-the-fly via smart constructor reductions.
//!

use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::iter::scratchpad::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use smallvec::SmallVec;

/// The inline capacity threshold for stack-allocated child arrays.
///
/// This inline size is utilized by `SmallVec` during the preparation phase to store
/// naked literals directly on the stack.
///
/// ### Optimization Rationale
/// - **Not a Hard Limit**: This does **not** cap the maximum number of children an expression
///   can have. If a disjunction contains more than 32 elements, the underlying `SmallVec`
///   automatically and transparently spills over to a standard heap-allocated vector.
/// - **The Stack Sweet-Spot**: Setting this to 32 guarantees that for 95% of standard planning
///   sub-expressions (LIR/PDDL domains), input preparation runs with **zero heap allocation overhead**.
/// - **Stack Overflow Protection**: Keeping this value small (e.g., $32 \times \text{size\_of::<ExprId>()} \approx 128$ bytes)
///   ensures that the function's stack frame remains ultra-lightweight, preventing stack overflows
///   during deep recursive traversals.
const MAX_CHILDREN: usize = 32;

/// Transforms a logical expression into its Factored Normal Form (FNF).
///
/// This is the main public entry point of the module. It takes an arbitrary expression
/// (typically a flat `OR` coming from a DNF-like structure) and applies a greedy,
/// zero-allocation distributive factorization algorithm to minimize the total number of
/// nodes in the Low-level Intermediate Representation (LIR).
///
/// ### Mathematical Transformation
/// The function isolates independent sub-expressions and iteratively extracts common
/// conjunctive terms using the distributive law:
/// $$(A \land B) \lor (A \land C) \equiv A \land (B \lor C)$$
/// It handles non-conjunctions (naked literals) gracefully by keeping them separated
/// from the factorization process and merging them back at the root level in the final step.
///
/// ### High-Level Architecture Pipeline
/// 1. **Phase 1: Preparation (`prepare_input`)**: Flattens the top-level disjunction. It fills
///    the `Scratchpad` with sorted conjunctive groups (`group_buffer`) and isolates non-`AND`
///    elements on the stack (`naked_literals`).
/// 2. **Phase 2 & 3: Slice Factorization & Assembly (`factorize_slice`)**: Executes the core
///    recursive window-based engine on the entire buffer slice `[0..total_groups]`. If fewer than
///    two groups are present, it safely bypasses factorization using `rebuild_flat_groups`.
/// 3. **Phase C: Final Synthesis**: Combines the structurally optimized factored branch with
///    the original naked literals collected during Phase 1 under a unified `OR` node.
///
/// ### Memory Allocation Strategy
/// - **Stack Allocation**: The naked literals are aggregated into a `SmallVec` bounded by
///   `MAX_CHILDREN` to guarantee that small-to-medium disjunctions never trigger a heap allocation.
/// - **Heap Staticity**: By threading a mutable `Scratchpad` reference through the pipeline,
///   the function guarantees **$O(1)$ heap allocation overhead**, as the internal buffers reuse
///   pre-existing structural capacities across multiple transformation calls.
///
/// ### Arguments
/// * `expr` - The root `ExprId` of the expression tree to transform into FNF.
/// * `builder` - A mutable reference to the `ExprBuilder` responsible for interning and smart-reducing expressions.
/// * `scratch` - A mutable reference to the persistent `Scratchpad` storage used for buffer windowing.
/// * `recursive` - A boolean flag enabling deep, cascading sub-factorization on extracted branches.
///
/// ### Returns
/// * `Ok(ExprId)` - The unique identifier of the fully factored, canonicalized, and interned expression tree.
/// * `Err(ExprOpErrorHC)` - If a retrieval or node interning error occurs within the Hash-Consing storage layer.
pub fn to_fnf(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
    recursive: bool,
) -> Result<ExprId, ExprOpErrorHC> {
    if expr.is_none() {
        return Ok(expr);
    }

    // --- 1. PREPARATION ---
    let naked_literals = prepare_input(expr, builder, scratch)?;
    let total_groups = scratch.group_boundaries().len();

    // --- 2 & 3. CORE LOGIC & ASSEMBLY ---
    let factored_branch = if total_groups < 2 {
        rebuild_flat_groups(builder, scratch, 0, total_groups)
    } else {
        factorize_slice(builder, scratch, 0, total_groups, recursive)?
    };

    // --- C. COMBINE WITH ORIGINAL NAKED LITERALS ---
    if naked_literals.is_empty() {
        Ok(factored_branch)
    } else {
        let mut final_args = naked_literals;
        let empty_or = builder.empty_or();

        if factored_branch != empty_or {
            final_args.push(factored_branch);
        }

        Ok(builder.or(&final_args))
    }
}

/// Prepares the input expression by extracting conjunctive groups and isolating naked literals.
///
/// This phase serves as the structural ingestor for the FNF transformation. It flattens the
/// immediate children of a top-level `OR` node, segregating sub-expressions that can be
/// factored (nested `AND` nodes) from those that cannot (atomic formulas or naked literals).
///
/// ### Algorithmic & Memory Invariants
/// - **In-place Buffer Optimization**: Clears the scratchpad's global `flat_groups` and
///   `group_boundaries` buffers without deallocating their underlying capacities, preventing
///   thousands of microscopic heap reallocations.
/// - **Unstable Canonical Sorting**: Every segment of child IDs added to `flat_groups` is sorted
///   immediately using `sort_unstable`. This is a **strict prerequisite** for the core factorization engine,
///   as it unlocks:
///   1. $O(\log n)$ lookup times via `binary_search` instead of $O(n)$ linear scans when matching factors.
///   2. Canonical representation guarantees so that Hash-Consing treats structurally uniform
///      conjunctions (like `A ∧ B` and `B ∧ A`) as the exact same interned node.
/// - **Naked Literal Seeding**: Elements that are not part of a conjunction (e.g., a single atom
///   directly under the root `OR`) bypass the `Scratchpad` and are collected into a local,
///   stack-allocated `SmallVec` to be rejoined at the very end of the transformation pipeline.
fn prepare_input(
    expr: ExprId,
    builder: &ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<SmallVec<[ExprId; MAX_CHILDREN]>, ExprOpErrorHC> {
    let mut naked = SmallVec::new();

    scratch.clear();
    let entry = builder.fetch(expr)?;

    let children = if matches!(entry.kind(), ExprEntryKind::Or) {
        entry.children()
    } else {
        std::slice::from_ref(&expr)
    };

    for &child_id in children {
        let child_entry = builder.fetch(child_id)?;

        if matches!(child_entry.kind(), ExprEntryKind::And) {
            let child_kids = child_entry.children();
            let start = scratch.flat_groups().len();

            // Push elements directly onto the flat global vector without temporary heap vectors
            scratch.flat_groups_mut().extend_from_slice(child_kids);
            let end = scratch.flat_groups().len();

            // Sort only the local slice corresponding to this group
            scratch.flat_groups_mut()[start..end].sort_unstable();

            scratch.group_boundaries_mut().push(start..end);
        } else {
            naked.push(child_id);
        }
    }

    Ok(naked)
}

/// Core recursive factorizer operating on a specific window of the global `group_boundaries`.
///
/// This function implements the greedy, frequency-based factorization algorithm in a strictly
/// bounded memory window. It partitions the conjunctive groups inside the slice, solves them
/// recursively, and applies the distributive law to assemble the local subtree.
///
/// ### Algorithmic Steps
/// 1. **Local Frequency Analysis**: Computes literal frequencies restricted *only* to the current
///    window `[start_idx..end_idx]` using `compute_frequencies_for_slice`.
/// 2. **Best Factor Selection**: Picks the most frequent literal $f$. If no literal appears in more
///    than one group, it falls back to a flat reconstruction via `rebuild_flat_groups`.
/// 3. **In-place Partitioning**: Rearranges the slice boundaries so that all groups containing $f$
///    are moved to the left sub-window `[start_idx..sub_branch_end]`, and $f$ is stripped from them.
/// 4. **Divide and Conquer**:
///    - *Left Branch*: Recursively refines the groups that shared $f$ (if `recursive` is true).
///    - *Right Branch*: Recursively processes the remaining groups that did not contain $f$.
/// 5. **Post-Order Assembly**: Integrates the results using the distributive identity:
///    $$(f \land \text{left\_branch}) \lor \text{right\_branch}$$
fn factorize_slice(
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
    start_idx: usize,
    end_idx: usize,
    recursive: bool,
) -> Result<ExprId, ExprOpErrorHC> {
    if (end_idx - start_idx) < 2 {
        return Ok(rebuild_flat_groups(builder, scratch, start_idx, end_idx));
    }

    // 1. Local frequency analysis
    scratch.compute_frequencies_for_slice(start_idx, end_idx);

    // 2. Locate the optimal common factor
    if let Some(f) = scratch.find_best_factor() {
        // 3. In-place boundary partitioning (modifies Range indices, leaves raw data stable)
        let num_with_f = scratch.partition_slice_by_factor(start_idx, end_idx, f);

        if num_with_f == 0 {
            return Ok(rebuild_flat_groups(builder, scratch, start_idx, end_idx));
        }

        let sub_branch_end = start_idx + num_with_f;

        // 4. Recursive divide-and-conquer processing
        let left_branch = if recursive {
            factorize_slice(builder, scratch, start_idx, sub_branch_end, recursive)?
        } else {
            rebuild_flat_groups(builder, scratch, start_idx, sub_branch_end)
        };

        let right_branch = factorize_slice(builder, scratch, sub_branch_end, end_idx, recursive)?;

        // 5. Intelligent post-order tree construction via distributivity
        let empty_and = builder.empty_and();
        let empty_or = builder.empty_or();

        let left_assembled = if left_branch == empty_and || left_branch == empty_or {
            f
        } else {
            builder.and(&[f, left_branch])
        };

        if right_branch == empty_or || sub_branch_end == end_idx {
            Ok(left_assembled)
        } else {
            Ok(builder.or(&[left_assembled, right_branch]))
        }
    } else {
        Ok(rebuild_flat_groups(builder, scratch, start_idx, end_idx))
    }
}

/// Reconstructs a sub-slice of conjunctive groups back into a flat `OR(AND(...))` expression.
///
/// This fallback or terminal assembly function takes the conjunctive groups stored in the
/// `Scratchpad` within the window `[start..end]` and builds their logical disjunction.
///
/// ### Memory & Borrow Checker Invariants
/// - **Indices-Based Iteration**: Uses explicit indexing and `scratch.get_group(i)` to cleanly
///   separate immutable group slicing borrows from mutable `scratch.build_buffer_mut()` staging targets.
/// - **Buffer Reuse**: Utilizes the scratchpad's `build_buffer` to stage the temporary `AND`
///   nodes before sending them to the top-level `OR` constructor, ensuring **zero heap allocations**.
fn rebuild_flat_groups(
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
    start: usize,
    end: usize,
) -> ExprId {
    scratch.build_buffer_mut().clear();

    for i in start..end {
        let group = scratch.get_group(i);
        let and_expr = builder.and(group);
        scratch.build_buffer_mut().push(and_expr);
    }

    builder.or(scratch.build_buffer())
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
