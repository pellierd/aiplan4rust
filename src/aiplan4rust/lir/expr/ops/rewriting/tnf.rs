//! # Temporal Normal Form (TNF) Transformation
//!
//! This module implements the transformation of PDDL temporal expressions into
//! **Temporal Normal Form (TNF)**.
//!
//! ## PDDL Semantic Handling
//!
//! The transformation adheres to PDDL 2.1+ standards regarding how time is distributed
//! across logical trees.
//!
//! ### 1. "Naked" Literals (Non-Temporal Expressions)
//! A "naked" literal is an atomic formula or a logical block that does not have an
//! explicit temporal qualifier (like `at start`).
//!
//! In this implementation, **naked literals are treated as implicit invariants**.
//! If a condition `P` is provided without a time specifier, it is replicated across
//! all three temporal fluxes (Start, End, and Overall). This ensures the condition
//! holds whenever the action is relevant.
//!
//! ### 2. Temporal Nesting (Error Detection)
//! PDDL semantics strictly forbid the nesting of temporal operators. Structures like
//! `(at start (at end P))` are logically undefined and physically nonsensical.
//!
//! During the descent phase, this module tracks the current temporal context. If a
//! temporal operator is encountered while the context is already set (i.e., not `None`),
//! the function returns an [`ExprOpErrorHC::IllegalTemporalNesting`].
//!
//! ### 3. Sparse Triplet Optimization
//! While TNF theoretically results in a triplet of (Start, End, Overall) expressions,
//! many actions only use a subset of these. This module implements **Sparse Reconstruction**:
//! * **Neutral Elements**: If a temporal flux is empty, it is replaced by the identity
//!   element (logical `True` via `empty_and`).
//! * **Pruning**: Using `rebuild_safe`, single-child logical nodes are collapsed,
//!   ensuring that the final TNF tree is as shallow as possible.
//!
//! ## Implementation Details: Bit-Packing Optimization
//!
//! To achieve high performance and avoid recursion-induced stack overflows, the
//! algorithm uses a manual stack-based traversal (via [`Scratchpad`]).
//!
//! A key optimization is the use of **Least Significant Bits (LSB) packing** to
//! track state without increasing the memory footprint of the stack:
//!
//! * **Context Packing (`TimeSpecifier`)**: The **two least significant bits** of the
//!   stored `usize` are reserved for the `TimeSpecifier` enum (None=0, Start=1, End=2, Overall=3).
//!   The `ExprId` is shifted left by 2 bits. This allows the algorithm to process the same
//!   sub-expression differently depending on its temporal path.
//! * **Phase Tracking**: The `Scratchpad` stores a boolean `processed` flag alongside
//!   the packed ID to distinguish between:
//!     1. **Descent Phase** (Pre-order): Context propagation and child discovery.
//!     2. **Reconstruction Phase** (Post-order): Synthesis of the temporal triplet
//!        from previously calculated child results.
//!
//! This bit-level manipulation allows the transformer to remain extremely fast
//! and memory-efficient, even for very large or deeply nested logical formulas.

use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::expr::iter::scratchpad::Scratchpad;
use crate::aiplan4rust::lir::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::expr::{ExprEntryKind, ExprId};
use smallvec::SmallVec;

/// The maximum number of children stored inline in a `SmallVec` before spilling to the heap.
/// This value is chosen to accommodate most PDDL logical expressions (And/Or)
/// without triggering frequent allocations.
const MAX_CHILDREN: usize = 32;

/// Transforms a temporal expression into its Temporal Normal Form (TNF).
///
/// TNF decomposes a PDDL expression into three distinct logical fluxes:
/// 1. **Start**: Conditions/Effects happening at the beginning of the action.
/// 2. **End**: Conditions/Effects happening at the end of the action.
/// 3. **Overall**: Invariants that must hold throughout the duration.
///
/// # Parameters
/// * `root`: The ID of the expression to transform.
/// * `builder`: The [`ExprBuilder`] used for fetching and interning nodes.
/// * `scratch`: A [`Scratchpad`] used for the non-recursive traversal and storing intermediate triplets.
///
/// # Returns
/// An `ExprId` representing an `(and (at start S) (at end E) (overall O))` node.
///
/// # Errors
/// Returns [`ExprOpErrorHC::IllegalTemporalNesting`] if a temporal operator is found
/// nested inside another temporal operator.
pub fn to_tnf(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    if expr.is_none() {
        return Ok(expr);
    }

    let empty = builder.empty_and();
    scratch.clear();

    // Buffer to store children IDs locally. Using SmallVec avoids heap allocation
    // for nodes with fewer than MAX_CHILDREN children.
    let mut children_ids: SmallVec<[ExprId; MAX_CHILDREN]> = SmallVec::new();

    // The root is initially processed with no temporal context (TimeSpecifier::None)
    let root_encoded = TimeSpecifier::None.pack(expr.as_usize());
    scratch.push(ExprId::from(root_encoded), false);

    while let Some((packed_id_wrapper, processed)) = scratch.pop() {
        let packed_id = packed_id_wrapper.as_usize();
        let (raw_id, ctx) = TimeSpecifier::unpack(packed_id);
        let curr_id = ExprId::from(raw_id);

        let kind = {
            let entry = builder.fetch(curr_id)?;
            children_ids.clear();
            children_ids.extend_from_slice(entry.children());
            entry.kind().clone()
        };

        if processed {
            // --- PHASE 2 : RECONSTRUCTION (Post-order traversal) ---
            let res_triplet = match &kind {
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                    // Extract the result from the child using its specific context
                    let child_ctx = TimeSpecifier::from_kind(&kind);
                    let child_packed = child_ctx.pack(children_ids[0].as_usize());
                    scratch.get_temporal_decomposition(child_packed)
                }

                ExprEntryKind::And
                | ExprEntryKind::Or
                | ExprEntryKind::Not
                | ExprEntryKind::Forall(_)
                | ExprEntryKind::Exists(_) => {
                    scratch.clear_time_specifier_buffers();

                    for &c in &children_ids {
                        // Children inherit the current context (ctx)
                        let child_packed = ctx.pack(c.as_usize());
                        let (s, e, o) = scratch.get_temporal_decomposition(child_packed);
                        scratch.push_time_specifier(s, e, o, empty);
                    }

                    // Rebuild logical nodes for each flux
                    let s_id = rebuild_safe(builder, &kind, scratch.at_start_buffer(), empty)?;
                    let e_id = rebuild_safe(builder, &kind, scratch.at_end_buffer(), empty)?;
                    let o_id = rebuild_safe(builder, &kind, scratch.overall_buffer(), empty)?;
                    (s_id, e_id, o_id)
                }

                _ => {
                    // Leaf nodes (Atoms, Fluents): Intern and assign to the correct flux based on context
                    let id = builder.intern(kind, &children_ids);
                    match ctx {
                        TimeSpecifier::AtStart => (id, empty, empty),
                        TimeSpecifier::AtEnd => (empty, id, empty),
                        TimeSpecifier::Overall => (empty, empty, id),
                        TimeSpecifier::None => (id, id, id), // Naked literals apply to all
                    }
                }
            };
            scratch.save_temporal_decomposition(packed_id, res_triplet);
        } else {
            // --- PHASE 1 : DESCENT (Pre-order traversal) ---
            let is_temporal = matches!(
                kind,
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall
            );

            // Illegal Nesting Detection: PDDL forbid nested temporal operators
            if ctx != TimeSpecifier::None && is_temporal {
                return Err(ExprOpErrorHC::illegal_temporal_nesting(
                    curr_id,
                    ctx.to_expr_kind(),
                    kind,
                ));
            }

            // Determine context for children
            let next_ctx = if is_temporal {
                TimeSpecifier::from_kind(&kind)
            } else {
                ctx
            };

            // Push back current node for post-order processing, then push children
            scratch.push(packed_id_wrapper, true);
            for &child in children_ids.iter().rev() {
                let child_packed = next_ctx.pack(child.as_usize());
                scratch.push(ExprId::from(child_packed), false);
            }
        }
    }

    // Final Assembly: Extract the root triplet (calculated under None context)
    let root_key = TimeSpecifier::None.pack(expr.as_usize());
    let (s, e, o) = scratch.get_temporal_decomposition(root_key);

    let nodes = [
        builder.at_start(s)?,
        builder.at_end(e)?,
        builder.overall(o)?,
    ];

    // Returns the final (and (at start S) (at end E) (overall O))
    Ok(builder.and(&nodes))
}

/// Reconstructs an expression node while applying basic logical simplifications.
///
/// This function acts as a safety layer during the upward pass of the TNF transformation.
/// It prevents the creation of redundant structures (like single-child logical nodes)
/// and ensures that empty collections are replaced by the appropriate identity element.
///
/// # Parameters
/// * `builder`: A mutable reference to the [`ExprBuilder`] used to intern the new node.
/// * `kind`: The [`ExprEntryKind`] of the node to reconstruct (e.g., `And`, `Or`, `Not`).
/// * `kids`: A slice of [`ExprId`] representing the already transformed children of this node.
/// * `empty`: The [`ExprId`] of the neutral element (logical True/empty AND) to return if `kids` is empty.
///
/// # Returns
/// * `Ok(ExprId)`: The ID of the simplified or reconstructed node.
/// * `Err(ExprOpErrorHC)`: If the builder fails to reconstruct the node.
///
/// # Optimizations
/// 1. **Empty Sets**: If `kids` is empty, it returns the `empty` ID (identity element).
/// 2. **Identity Law**: For `And` and `Or` nodes, if there is exactly one child, the node
///    is bypassed and the child's ID is returned directly.
/// 3. **Canonicalization**: In all other cases, it delegates to `builder.reconstruct`,
///    which handles structural deduplication (hash-consing).
fn rebuild_safe(
    builder: &mut ExprBuilder,
    kind: &ExprEntryKind,
    kids: &[ExprId],
    empty: ExprId,
) -> Result<ExprId, ExprOpErrorHC> {
    if kids.is_empty() {
        // Identity: empty AND/OR often simplifies to True/False in TNF contexts
        Ok(empty)
    } else if kids.len() == 1 && matches!(kind, ExprEntryKind::And | ExprEntryKind::Or) {
        // Simplify: AND(A) -> A, OR(A) -> A
        Ok(kids[0])
    } else {
        // General case: call the builder to intern the node
        Ok(builder.reconstruct(kind.clone(), kids)?)
    }
}

/// Checks if the given expression is "fully temporal," meaning every atomic formula
/// is enclosed within at least one temporal operator (`at start`, `at end`, or `overall`).
///
/// In PDDL, a "naked" literal (an atom without a temporal qualifier) is often
/// interpreted as an invariant or as being required across multiple time points.
/// This function is used to identify such expressions before or during TNF transformation.
///
/// # Parameters
/// * `root`: The [`ExprId`] of the expression tree to analyze.
/// * `builder`: A reference to the [`ExprBuilder`] used to fetch node data.
/// * `scratch`: A mutable reference to a [`Scratchpad`] used for the non-recursive
///   depth-first search (DFS) traversal.
///
/// # Returns
/// * `Ok(true)` if every terminal node (atom/fluent) is covered by a temporal specifier.
/// * `Ok(false)` if at least one "naked" literal is found.
/// * `Err(ExprOpErrorHC)` if a node cannot be fetched from the store.
///
/// # Implementation Details
/// This function performs a manual stack-based traversal to avoid recursion limits.
/// It uses bit-packing on the [`ExprId`] stored in the scratchpad:
/// - The upper bits store the actual `ExprId`.
/// - The least significant bit (LSB) acts as a boolean flag: `1` if the current node
///   is descendants of a temporal operator, `0` otherwise.
pub fn is_fully_temporal(
    root: ExprId,
    builder: &ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<bool, ExprOpErrorHC> {
    scratch.clear();

    // Pack the root: ID in upper bits, "under_temporal" flag (0) in the LSB.
    let root_packed = root.as_usize() << 1;
    scratch.push(ExprId::from(root_packed), false);

    while let Some((packed_id, _)) = scratch.pop() {
        let val = packed_id.as_usize();
        let curr_id = ExprId::from(val >> 1);
        let is_under_temporal = (val & 1) == 1;

        let entry = builder.fetch(curr_id)?;
        let kind = entry.kind();

        match kind {
            // Temporal Operators: Set the flag to 1 for all children.
            ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                for &child in entry.children() {
                    let next_packed = (child.as_usize() << 1) | 1;
                    scratch.push(ExprId::from(next_packed), false);
                }
            }
            // Logical Connectives & Quantifiers: Propagate the current flag to children.
            ExprEntryKind::And
            | ExprEntryKind::Or
            | ExprEntryKind::Not
            | ExprEntryKind::Forall(_)
            | ExprEntryKind::Exists(_) => {
                let flag = if is_under_temporal { 1 } else { 0 };
                for &child in entry.children() {
                    let next_packed = (child.as_usize() << 1) | flag;
                    scratch.push(ExprId::from(next_packed), false);
                }
            }
            // Terminal Nodes (Atoms, Fluents, etc.)
            _ => {
                if !is_under_temporal {
                    // Found a leaf node that is not wrapped in any temporal operator.
                    return Ok(false);
                }
            }
        }
    }

    // All atoms encountered were under a temporal operator.
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
/// Represents the temporal context of an expression in PDDL 2.1+.
///
/// This enum is used during the Temporal Normal Form (TNF) transformation to track
/// whether an expression belongs to the start, end, or invariant (overall) flux of an action.
pub enum TimeSpecifier {
    /// No temporal context. Often represents a "naked" literal that applies to all fluxes.
    None = 0,
    /// Corresponds to the `(at start ...)` PDDL operator.
    AtStart = 1,
    /// Corresponds to the `(at end ...)` PDDL operator.
    AtEnd = 2,
    /// Corresponds to the `(overall ...)` PDDL operator.
    Overall = 3,
}

impl TimeSpecifier {
    /// Mask used to extract the 2-bit temporal specifier from a packed `usize`.
    const BITS_MASK: usize = 0x3;

    /// Number of bits used by the specifier, used for shifting the primary value (e.g., ExprId).
    const SHIFT: usize = 2;

    /// Packs a value (typically an `ExprId` as a `usize`) with this temporal specifier.
    ///
    /// This allows storing both the ID and its temporal context in a single `usize`
    /// to optimize memory usage in the `Scratchpad` during tree traversal.
    ///
    /// # Parameters
    /// * `id_val`: The raw numerical value (ID) to be packed.
    ///
    /// # Returns
    /// A `usize` containing the shifted ID in the upper bits and the specifier in the lower 2 bits.
    #[inline(always)]
    fn pack(self, id_val: usize) -> usize {
        (id_val << Self::SHIFT) | (self as usize)
    }

    /// Unpacks a `usize` into its original raw ID and its `TimeSpecifier` context.
    ///
    /// # Parameters
    /// * `packed_val`: The composite value previously created by [`Self::pack`].
    ///
    /// # Returns
    /// A tuple containing:
    /// 1. The original `usize` ID.
    /// 2. The recovered `TimeSpecifier` enum variant.
    #[inline(always)]
    fn unpack(packed_val: usize) -> (usize, Self) {
        let id_val = packed_val >> Self::SHIFT;
        let spec = match packed_val & Self::BITS_MASK {
            1 => Self::AtStart,
            2 => Self::AtEnd,
            3 => Self::Overall,
            _ => Self::None,
        };
        (id_val, spec)
    }

    /// Converts an [`ExprEntryKind`] into its corresponding [`TimeSpecifier`].
    ///
    /// This is used during the downward pass of the TNF algorithm to update
    /// the current temporal context based on the node being visited.
    ///
    /// # Parameters
    /// * `kind`: A reference to the expression entry kind.
    ///
    /// # Returns
    /// The matching `TimeSpecifier` if `kind` is a temporal operator; otherwise `TimeSpecifier::None`.
    fn from_kind(kind: &ExprEntryKind) -> Self {
        match kind {
            ExprEntryKind::AtStart => Self::AtStart,
            ExprEntryKind::AtEnd => Self::AtEnd,
            ExprEntryKind::Overall => Self::Overall,
            _ => Self::None,
        }
    }

    /// Rebuilds an [`ExprEntryKind`] from this specifier.
    ///
    /// Primarily used for error reporting (e.g., in `IllegalTemporalNesting`)
    /// or when reconstructing the temporal wrapper nodes.
    ///
    /// # Returns
    /// The [`ExprEntryKind`] variant corresponding to this specifier.
    ///
    /// # Panics
    /// Panics if called on [`TimeSpecifier::None`], as there is no
    /// corresponding temporal operator node for a null context.
    fn to_expr_kind(&self) -> ExprEntryKind {
        match self {
            Self::AtStart => ExprEntryKind::AtStart,
            Self::AtEnd => ExprEntryKind::AtEnd,
            Self::Overall => ExprEntryKind::Overall,
            Self::None => {
                unreachable!("Cannot convert TimeSpecifier::None to an ExprEntryKind context")
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, TypedSymbol, VariableId};
    use crate::aiplan4rust::lir::expr::ExprStore;

    /// Helper to verify the top-level structure of a TNF (Temporal Normal Form) result.
    ///
    /// TNF transformation should result in a logical AND node containing three specific
    /// temporal components: AtStart, AtEnd, and Overall. This function handles builder
    /// optimizations where empty components (logical True) might be simplified.
    fn verify_tnf_structure(
        builder: &mut ExprBuilder,
        root_id: ExprId,
    ) -> (ExprId, ExprId, ExprId) {
        // 1. Retrieve the neutral element (logical True) from the builder
        let empty = builder.empty_and();

        // 2. Fetch the root node and clone its children IDs immediately.
        // By using .to_vec(), we own the IDs and release the immutable borrow on the builder,
        // allowing us to perform further mutable or immutable operations on it.
        let (root_kind, children_ids) = {
            let root = builder.fetch(root_id).expect("Root node missing in store");
            (root.kind().clone(), root.children().to_vec())
        };

        assert!(
            matches!(root_kind, ExprEntryKind::And),
            "TNF root must be an 'And' node, but found: {:?}",
            root_kind
        );

        let mut s = empty;
        let mut e = empty;
        let mut o = empty;

        // 3. Iterate through the children to assign them to their respective temporal slots
        for child_id in children_ids {
            let child_node = builder
                .fetch(child_id)
                .expect("Child node missing in store");
            let child_kind = child_node.kind();

            // Extract the inner expression ID from the temporal specifier (e.g., 'A' in 'AtStart(A)')
            // If the child is an optimized node (like an empty AND), we debug to the 'empty' ID.
            let inner_id = child_node.children().get(0).copied().unwrap_or(empty);

            match child_kind {
                ExprEntryKind::AtStart => s = inner_id,
                ExprEntryKind::AtEnd => e = inner_id,
                ExprEntryKind::Overall => o = inner_id,
                _ => {
                    // Ignore optimized nodes or neutral elements that don't match
                    // a specific temporal specifier.
                }
            }
        }

        (s, e, o)
    }

    /// Tests the decomposition of explicit temporal operators.
    /// - **Input**: `(AND (at start A) (at end B) (overall C))`
    /// - **Expected Output**: A TNF structure where:
    ///   - Start component is exactly `A`
    ///   - End component is exactly `B`
    ///   - Overall component is exactly `C`
    #[test]
    fn test_tnf_simple_atoms() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Create atoms
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        // 2. Create temporal nodes (one by one to satisfy the borrow checker)
        let ts = builder.at_start(a)?;
        let te = builder.at_end(b)?;
        let to = builder.overall(c)?;

        // 3. Final assembly
        let expr = builder.and(&[ts, te, to]);

        // 4. Transform
        let result_id = to_tnf(expr, &mut builder, &mut scratch)?;

        // 5. Validation
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);
        assert_eq!(s, a);
        assert_eq!(e, b);
        assert_eq!(o, c);

        Ok(())
    }

    /// Tests how the algorithm handles "naked" literals (without time specifiers).
    /// - **Input**: `(AND A (at start B) (at end C))`
    /// - **Expected Output**: `A` is treated as `TimeSpecifier::None` and propagated to all fluxes.
    ///   - Start: `(AND A B)`
    ///   - End: `(AND A C)`
    ///   - Overall: `A` (Simplified from `AND(A)`)
    #[test]
    fn test_tnf_mixed_temporal_logic() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        let ts = builder.at_start(b)?;
        let te = builder.at_end(c)?;

        // Setup: (and A (at start B) (at end C))
        let expr = builder.and(&[a, ts, te]);

        // Transform
        let result_id = to_tnf(expr, &mut builder, &mut scratch)?;

        // Extract components (S, E, O)
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);

        // --- VALIDATION START (S) ---
        let s_node = builder.fetch(s)?;
        let s_kids = s_node.children();
        assert!(s_kids.contains(&a), "Start should contain A");
        assert!(s_kids.contains(&b), "Start should contain B");

        // --- VALIDATION END (E) ---
        let e_node = builder.fetch(e)?;
        let e_kids = e_node.children();
        assert!(e_kids.contains(&a), "End should contain A");
        assert!(e_kids.contains(&c), "End should contain C");

        // --- VALIDATION OVERALL (O) ---
        // Since only 'A' is None, the Overall flux contains only 'A'.
        // The builder simplifies AND(A) -> A.
        if o != a {
            let o_node = builder.fetch(o)?;
            assert!(o_node.children().contains(&a), "Overall should contain A");
        }

        Ok(())
    }

    /// Tests the idempotency of the TNF transformation.
    /// - **Input**: Any valid expression `E`
    /// - **Expected Output**: `to_tnf(to_tnf(E)) == to_tnf(E)`
    ///   The transformation should be stable and not duplicate temporal wrappers on multiple passes.
    #[test]
    fn test_tnf_idempotency() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let root = builder.at_start(a)?;

        let result1 = to_tnf(root, &mut builder, &mut scratch)?;
        let result2 = to_tnf(result1, &mut builder, &mut scratch)?;

        assert_eq!(result1, result2, "TNF must be stable and idempotent");
        Ok(())
    }

    /// Tests the detection of fully temporal formulas.
    /// - **Input 1**: `(at start A)` -> Expected: `true`
    /// - **Input 2**: `(and (at start A) B)` -> Expected: `false` (B is "naked")
    /// - **Input 3**: `(at start (and A B))` -> Expected: `true` (A and B inherit context)
    #[test]
    fn test_is_fully_temporal() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);

        // Case 1: Simple temporal
        let pure_t = builder.at_start(a)?;
        assert!(is_fully_temporal(pure_t, &builder, &mut scratch)?);

        // Case 2: Mixed (B is not covered)
        let at_a = builder.at_start(a)?;
        let mixed = builder.and(&[at_a, b]);
        assert!(!is_fully_temporal(mixed, &builder, &mut scratch)?);

        // Case 3: Block temporal
        let and_ab = builder.and(&[a, b]);
        let block_t = builder.at_start(and_ab)?;
        assert!(is_fully_temporal(block_t, &builder, &mut scratch)?);

        Ok(())
    }

    /// Tests TNF with negations to ensure the temporal context is pushed through.
    /// - **Input**: `(at start (not A))`
    /// - **Expected Output**: Start component should be `(not A)`.
    #[test]
    fn test_tnf_with_negation() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let not_a = builder.not(a);
        let expr = builder.at_start(not_a)?;

        let result_id = to_tnf(expr, &mut builder, &mut scratch)?;
        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);

        let s_node = builder.fetch(s)?;
        assert!(matches!(s_node.kind(), ExprEntryKind::Not));
        assert_eq!(s_node.children()[0], a);

        Ok(())
    }

    /// Tests the intelligent filtering of unused quantified variables.
    /// - **Input**: `(at start (forall {?x} P))` where `?x` is NOT used in `P`.
    /// - **Result**: The `Forall` is stripped by the builder; TNF handles the naked atom.
    #[test]
    fn test_tnf_quantifier_filtering() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let ty = builder.ty(&[1]);
        let var_x = TypedSymbol::new(VariableId::from(10), ty);
        let vars = builder.typed_variable_list(vec![var_x]);

        // P does NOT use variable 10
        let p = builder.atomic_formula(1, &[], skel);
        let forall = builder.forall(vars, p)?; // Builder returns `p` directly here

        let root = builder.at_start(forall)?;
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);
        assert_eq!(
            s, p,
            "The Forall should have been filtered out because ?x is not free in p"
        );

        Ok(())
    }

    /// Tests the preservation of a valid quantifier through the TNF transformation.
    /// - **Input**: `(at start (forall {?x} (P ?x)))`
    /// - **Result**: The Start component correctly contains the original Forall node.
    #[test]
    fn test_tnf_quantifier_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let ty = builder.ty(&[1]);
        let var_id = VariableId::from(10);
        let var_symbol = TypedSymbol::new(var_id, ty);
        let vars = builder.typed_variable_list(vec![var_symbol]);

        // FIX: Use builder.variable(id) to create a valid ExprId leaf
        let var_expr = builder.variable(var_id);

        // P USES variable 10, so the forall is preserved by the builder
        let p_x = builder.atomic_formula(1, &[var_expr], skel);
        let forall = builder.forall(vars, p_x)?;

        let root = builder.at_start(forall)?;
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);
        let s_node = builder.fetch(s)?;

        assert!(
            matches!(s_node.kind(), ExprEntryKind::Forall(_)),
            "Forall node must be present"
        );
        assert_eq!(
            s_node.children()[0],
            p_x,
            "The Forall body must match the original atomic formula p_x"
        );

        Ok(())
    }

    /// Tests that quantifier flattening is maintained during TNF reconstruction.
    /// - **Input**: `(at start (forall {?x} (forall {?y} P(x,y))))`
    /// - **Result**: The Start component is a single Forall node containing both variables.
    #[test]
    fn test_tnf_quantifier_flattening() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let t1 = builder.ty(&[1]);
        let v1_id = VariableId::from(1);
        let v1_sym = TypedSymbol::new(v1_id, t1);

        let t2 = builder.ty(&[2]);
        let v2_id = VariableId::from(2);
        let v2_sym = TypedSymbol::new(v2_id, t2);

        // Create valid variable leaf expressions
        let v1_expr = builder.variable(v1_id);
        let v2_expr = builder.variable(v2_id);

        let body = builder.atomic_formula(1, &[v1_expr, v2_expr], skel);

        // The builder will flatten these two calls into a single Forall({v1, v2}, body)
        let var1s = builder.typed_variable_list(vec![v1_sym]);
        let inner = builder.forall(var1s, body)?;
        let var2s = builder.typed_variable_list(vec![v2_sym]);
        let outer = builder.forall(var2s, inner)?;

        let root = builder.at_start(outer)?;
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);
        let s_node = builder.fetch(s)?;

        if let ExprEntryKind::Forall(vars) = s_node.kind() {
            assert_eq!(
                vars.len(),
                2,
                "Quantifiers should remain flattened (v1 and v2) in the TNF output"
            );
        } else {
            panic!("Expected a Forall node, but received: {}", s_node.kind());
        }

        Ok(())
    }

    /// Tests the empty/identity case.
    /// - **Input**: `(at start (and))` (an empty AND)
    /// - **Expected Output**: All components should be the `empty_and` (Logical True).
    #[test]
    fn test_tnf_empty_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let empty = builder.empty_and();

        let root = builder.at_start(empty)?;
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);
        assert_eq!(s, empty);
        assert_eq!(e, empty);
        assert_eq!(o, empty);

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that a conditional effect (`WHEN`) is correctly handled
    /// during TNF transformation when wrapped in a temporal operator.
    ///
    /// **Input**: A temporal expression containing a conditional effect: `(at start (when A B))`.
    ///
    /// **Expected Output**: A TNF structure where the `start` component `S` is the
    /// original conditional effect `(when A B)`. The `A` (condition) and `B` (effect)
    /// must remain as direct children of the `WHEN` node.
    fn test_tnf_conditional_effect() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Create atoms for condition (A) and effect (B)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);

        // 2. Create the conditional effect node: (when A B)
        let when = builder.when(a, b);

        // 3. Wrap it in a temporal context: (at start (when A B))
        let root = builder.at_start(when)?;

        // 4. Transform to TNF
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        // 5. Verify the top-level (AND S E O) structure and extract S
        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);

        // 6. Assertions: The start component must be the original WHEN node
        let s_node = builder.fetch(s)?;
        assert!(
            matches!(s_node.kind(), ExprEntryKind::When),
            "The Start component should be a WHEN node, but found: {:?}",
            s_node.kind()
        );

        // Verify that children (condition and effect) are preserved in order
        assert_eq!(
            s_node.children()[0],
            a,
            "The condition (A) is missing or displaced"
        );
        assert_eq!(
            s_node.children()[1],
            b,
            "The effect (B) is missing or displaced"
        );

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies the distribution of a temporal operator over a logical `OR` node.
    /// In TNF, `(at start (A or B))` should result in the `OR` being preserved within the
    /// `start` component of the decomposition.
    ///
    /// **Input**: A temporal expression wrapping a logical disjunction: `(at start (A or B))`.
    ///
    /// **Expected Output**: A TNF structure where the `start` component `S` is exactly `(A or B)`.
    /// The `end` and `overall` components should be empty (logical true).
    fn test_tnf_or_distribution() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Create atoms A and B
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);

        // 2. Create the logical OR: (A or B)
        let or_node = builder.or(&[a, b]);

        // 3. Wrap in a temporal operator: (at start (A or B))
        let root = builder.at_start(or_node)?;

        // 4. Transform to TNF
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        // 5. Verify structure and extract the Start component
        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);

        // 6. Assertions: The start component must be the OR node
        let s_node = builder.fetch(s)?;
        assert!(
            matches!(s_node.kind(), ExprEntryKind::Or),
            "The Start component should be an OR node, but found: {:?}",
            s_node.kind()
        );

        let s_kids = s_node.children();
        assert!(s_kids.contains(&a));
        assert!(s_kids.contains(&b));

        Ok(())
    }

    #[test]
    /// **Purpose**: Ensures that the system strictly forbids nested temporal operators
    /// (e.g., `at start (at end A)`), as they are semantically invalid in PDDL 2.1+.
    ///
    /// **Input**: A nested temporal expression where a temporal operator is the direct child
    /// of another temporal operator.
    ///
    /// **Expected Output**: The test succeeds if either:
    /// 1. The `ExprBuilder` prevents the creation of the expression (returning an error).
    /// 2. The `to_tnf` function detects the nesting and returns an `IllegalTemporalNesting` error.
    /// It fails if the expression is processed as a valid TNF.
    fn test_tnf_nested_temporal_conflict() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Create a simple atom
        let a = builder.atomic_formula(1, &[], skel);

        // 2. Create the first level (at end A) -> Should be OK
        let inner = builder.at_end(a)?;

        // 3. Attempt to nest: (at start (at end A))
        // We check both the builder and the TNF algorithm for safety.

        // CASE A: The builder proactively rejects the nesting
        let outer_res = builder.at_start(inner);

        if let Err(e) = outer_res {
            // Success: The builder prevented the invalid structure
            println!("Builder correctly rejected nesting: {:?}", e);
            return Ok(());
        }

        // CASE B: The builder allowed it, so to_tnf must catch it
        let outer = outer_res.unwrap();
        let result = to_tnf(outer, &mut builder, &mut scratch);

        match result {
            Err(ExprOpErrorHC::IllegalTemporalNesting { .. }) => {
                // Success: TNF algorithm detected the illegal nesting
                Ok(())
            }
            Ok(_) => {
                panic!("TNF should have rejected nesting (at start (at end ...))");
            }
            Err(e) => {
                panic!("Expected IllegalTemporalNesting error, but got: {:?}", e);
            }
        }
    }

    #[test]
    /// **Purpose**: Verifies that a pure `overall` constraint is correctly isolated
    /// into the `overall` flux of the TNF, leaving the `start` and `end` fluxes empty.
    ///
    /// **Input**: A single temporal expression `(overall A)`.
    ///
    /// **Expected Output**: A TNF structure `(and (at start true) (at end true) (overall A))`.
    /// After optimization by the builder, the `start` and `end` components should
    /// match the `empty_and` (logical true) and the `overall` component should match `A`.
    fn test_tnf_overall_isolation() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Create the input: (overall A)
        let a = builder.atomic_formula(1, &[], skel);
        let root = builder.overall(a)?;

        // 2. Transform to TNF
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        // 3. Verify structure and extract S, E, O components
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);

        let empty = builder.empty_and();

        // 4. Assertions
        assert_eq!(
            s, empty,
            "Start should be empty for a pure Overall constraint"
        );
        assert_eq!(
            e, empty,
            "End should be empty for a pure Overall constraint"
        );
        assert_eq!(o, a, "Overall should contain the atom");

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that negation wrapping a logical block is preserved
    /// within its temporal flux.
    ///
    /// **Input**: `(at start (not (and A B)))`
    ///
    /// **Expected Output**: Start component `S` should be exactly `(not (and A B))`.
    fn test_tnf_negated_block() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let and_ab = builder.and(&[a, b]);
        let not_and = builder.not(and_ab);
        let root = builder.at_start(not_and)?;

        let result_id = to_tnf(root, &mut builder, &mut scratch)?;
        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);

        let s_node = builder.fetch(s)?;
        assert!(matches!(s_node.kind(), ExprEntryKind::Not));

        let inner_and = builder.fetch(s_node.children()[0])?;
        assert!(matches!(inner_and.kind(), ExprEntryKind::And));
        assert_eq!(inner_and.children().len(), 2);

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that a single logical AND containing different
    /// temporal specifiers is correctly decomposed into its respective fluxes.
    ///
    /// **Input**: `(and (at start A) (overall B))`
    ///
    /// **Expected Output**: `S` contains `A`, `O` contains `B`, `E` is empty.
    fn test_tnf_split_and_logic() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let ts = builder.at_start(a)?;
        let to = builder.overall(b)?;
        let root = builder.and(&[ts, to]);

        let result_id = to_tnf(root, &mut builder, &mut scratch)?;
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);

        assert_eq!(s, a, "Start flux should contain A");
        assert_eq!(o, b, "Overall flux should contain B");
        assert_eq!(e, builder.empty_and(), "End flux should be empty");

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that a quantifier wrapping a temporal operator
    /// is correctly handled (the temporal operator is pushed up or preserved).
    /// Note: In PDDL, `(forall (?x) (at start (P ?x)))` is common.
    ///
    /// **Input**: `(forall {?x} (at start (P ?x)))`
    ///
    /// **Expected Output**: `S` should contain `(forall {?x} (P ?x))`.
    fn test_tnf_quantifier_over_temporal() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let ty = builder.ty(&[1]);
        let var_id = VariableId::from(10);
        let var_sym = TypedSymbol::new(var_id, ty);
        let vars = builder.typed_variable_list(vec![var_sym]);

        let var = builder.variable(var_id);
        let p_x = builder.atomic_formula(1, &[var], skel);
        let at_start_p = builder.at_start(p_x)?;
        let root = builder.forall(vars, at_start_p)?;

        let result_id = to_tnf(root, &mut builder, &mut scratch)?;
        let (s, _, _) = verify_tnf_structure(&mut builder, result_id);

        let s_node = builder.fetch(s)?;
        assert!(matches!(s_node.kind(), ExprEntryKind::Forall(_)));
        assert_eq!(s_node.children()[0], p_x);

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that a "naked" atom (no temporal operator) inside a
    /// complex logical tree is correctly replicated across all three TNF fluxes.
    ///
    /// **Input**: `(at start A) and (B or C)` where B and C are naked.
    ///
    /// **Expected Output**:
    /// - Start: `A and (B or C)`
    /// - End: `B or C`
    /// - Overall: `B or C`
    fn test_tnf_deep_naked_atom_propagation() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        let at_start_a = builder.at_start(a)?;
        let or_bc = builder.or(&[b, c]);
        let root = builder.and(&[at_start_a, or_bc]);

        let result_id = to_tnf(root, &mut builder, &mut scratch)?;
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);

        // Verify Start: should contain both A and the OR block
        let s_node = builder.fetch(s)?;
        assert_eq!(
            s_node.children().len(),
            2,
            "Start should have 2 children (A and OR block)"
        );

        // Verify End and Overall: should contain the OR block (B or C)
        assert_eq!(
            e, or_bc,
            "End flux should have inherited the naked OR block"
        );
        assert_eq!(
            o, or_bc,
            "Overall flux should have inherited the naked OR block"
        );

        Ok(())
    }

    #[test]
    /// **Purpose**: Tests the handling of redundant or empty temporal blocks.
    ///
    /// **Input**: `(and (at start (and)) (at end A))`
    ///
    /// **Expected Output**: The `at start` component should be a clean `empty_and`.
    fn test_tnf_redundant_empty_blocks() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let empty_and = builder.empty_and();
        let at_start_empty = builder.at_start(empty_and)?;
        let at_end_a = builder.at_end(a)?;

        let root = builder.and(&[at_start_empty, at_end_a]);

        let result_id = to_tnf(root, &mut builder, &mut scratch)?;
        let (s, e, o) = verify_tnf_structure(&mut builder, result_id);

        assert_eq!(s, empty_and, "Start should be simplified to empty_and");
        assert_eq!(e, a, "End should contain A");
        assert_eq!(o, empty_and, "Overall should be empty_and");

        Ok(())
    }

    #[test]
    /// **Purpose**: Verifies that the bit-packing logic in TimeSpecifier doesn't
    /// fail with high-index ExprIds (stressing the shift logic).
    ///
    /// **Input**: A temporal expression using an ID that is large (e.g., 1000000).
    fn test_tnf_high_id_bit_packing() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // Manually create a high ID by pushing dummy atoms
        let mut last_id = builder.empty_and();
        for i in 0..1000 {
            last_id = builder.atomic_formula(i, &[], skel);
        }

        let root = builder.overall(last_id)?;
        let result_id = to_tnf(root, &mut builder, &mut scratch)?;

        let (_, _, o) = verify_tnf_structure(&mut builder, result_id);
        assert_eq!(
            o, last_id,
            "The high-index ID should be perfectly preserved through bit-packing"
        );

        Ok(())
    }
}
