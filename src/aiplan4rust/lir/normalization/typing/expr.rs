//! # Expression Tree Normalization (Typing Pass)
//!
//! This module implements a non-recursive, double-stack-based Postorder traversal
//! to resolve and unify type signatures within quantifier scopes (`Forall` and `Exists`).
//!
//! ## Hash-Consing & Immutability
//! Because the central `ExprStore` enforces structural uniqueness and immutability,
//! modifying an attribute (like the variable type list of a quantifier) requires
//! creating a new node and propagating its new `ExprId` upwards to its parents.
//!
//! ## Memory Optimization & Architectural Flow
//! To ensure high performance and prevent stack overflows on deep expression trees,
//! this module avoids recursion entirely. It coordinates with an external, reusable
//! [`Scratchpad`] fetched directly from the infrastructure module (`lir::old::expr::iter`).
//!
//! By operating via the scratchpad's internal contiguous buffers, the normalization pipeline
//! unrolls the tree traversal into an iterative downward discovery and bottom-up reconstruction pass.
//! This ensures a **$\mathcal{O}(1)$ dynamic allocation profile** at runtime, drastically limiting
//! cache misses and freeing the CPU from intermediate vector allocations.

use crate::aiplan4rust::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprEntryKind, ExprId, ExprStore};
use crate::aiplan4rust::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::lir::normalization::typing::{typed_symbol, TypeRegistry};

/// Normalizes type signatures within quantifier scopes throughout an expression tree.
///
/// # Semantic Goal & Motivation (What & Why)
/// In PDDL/HDDL planning domains, quantifiers (`Forall` and `Exists`) introduce scoped
/// variables that can possess complex, compound, or implicit type signatures (e.g.,
/// hierarchical types, or `either` types like `?x - (either type_a type_b)`).
///
/// This normalization pass performs two critical tasks:
/// 1. **Type Resolution & Unification**: It evaluates and resolves these complex type expressions
///    against the canonical [`TypeRegistry`], flattening or unifying them into a standardized,
///    primitive representation that the rest of the compiler can understand.
/// 2. **Downstream Preparation (Grounding)**: By guaranteeing that every scoped variable
///    has a fully resolved, concrete type signature *before* leaving this pass, it ensures that
///    the downstream **Grounder** can efficiently instantiate quantifiers into finite
///    conjunctions/disjunctions without having to re-evaluate type inheritance or compound types at runtime.
///
/// # Principle & Algorithm (How)
/// This function performs a type-normalization pass over an expression tree. Because the underlying
/// `ExprStore` utilizes immutable, hash-consed structural nodes, updating any part of a node
/// (such as variables inside quantifiers) requires structural replication and bottom-up propagation
/// of newly generated [`ExprId`]s up to the root.
///
/// To prevent stack overflows on deep expression trees and eliminate runtime memory allocations,
/// the algorithm avoids recursion entirely. Instead, it coordinates a non-recursive, double-stack
/// **DFS Post-order Traversal** using an external pre-allocated [`Scratchpad`].
///
/// The execution is split into two distinct interleaved states per node:
/// 1. **Descent Phase (`processed = false`)**: Discovers unvisited nodes, marks them, records their
///    structural children into a contiguous scratch buffer without allocations, and pushes them onto
///    the evaluation stack in reverse order to preserve semantic evaluation sequences.
/// 2. **Reconstruction Phase (`processed = true`)**: Executes bottom-up during stack unwinding.
///    It reads already-translated child IDs from the cache, resolves or updates any local type unifications
///    via the [`TypeRegistry`] (specifically tracking `Forall` and `Exists` scopes), and requests
///    the [`ExprBuilder`] to reconstruct the structurally unique node.
///
/// # Complexity Analysis
/// Let $N$ be the total number of nodes in the sub-tree rooted at `root`, and $E$ be the total number of edges.
/// Let $V$ be the maximum number of scoped variables declared inside a single quantifier node.
///
/// - **Time Complexity**: $\mathcal{O}(N + N \cdot \log V)$ worst-case.
///   - Traversing and reconstructing the tree structure takes $\mathcal{O}(N)$ since every node is
///     discovered and processed exactly once. Linearization reads and writes via the scratchpad are
///     guaranteed $\mathcal{O}(1)$ flat memory moves (`memcpy`).
///   - Within quantifier nodes (`Forall`/`Exists`), type resolution per variable interacts with the
///     `TypeRegistry`. If registry unifications or lookups scale logarithmically with variable count,
///     this adds an extra $\mathcal{O}(\log V)$ factor per variable.
///   - Lookup and insertion into the `Scratchpad` cache take $\mathcal{O}(1)$ average time via `FxHashMap`.
///
/// - **Space Complexity**: $\mathcal{O}(D)$ auxiliary space, where $D$ is the maximum depth of the tree.
///   - The memory consumption on the heap is bounded by the size of the `Scratchpad` buffers.
///     The stack depth never exceeds $\mathcal{O}(D)$ elements simultaneously.
///   - Crucially, this operation guarantees **$\mathcal{O}(1)$ dynamic allocations** at runtime,
///     as it strictly borrows and recycles the structural capacity from the caller's `scratch` instance.
///
/// # Arguments
/// * `root` - The initial root identifier of the expression tree to normalize.
/// * `old` - A mutable reference to the global expression old hosting the node entries.
/// * `registry` - A mutable reference to the environment's canonical type definitions and unifier.
/// * `scratch` - An external, reusable memory arena tracking the traversal state, buffers, and ID translation cache.
///
/// # Errors
/// Returns a [`NormalizationError`] if:
/// - A corrupted graph structure is detected (e.g., an expected child ID or the final reconstructed
///   root is missing from the translation cache).
/// - Type unification fails or a symbol violates registry invariants.
/// - The `ExprBuilder` fails to allocate slot IDs in the `ExprStore` during hash-consed reconstruction.
pub fn normalize(
    root: ExprId,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
    scratch: &mut Scratchpad,
) -> Result<ExprId, NormalizationError> {
    let mut builder = ExprBuilder::new(store);

    // Initialize the scratchpad for a new traversal pass
    scratch.init(root);

    // Non-recursive DFS Post-order traversal loop
    while let Some((current_id, processed)) = scratch.pop() {
        if processed {
            // --- RECONSTRUCTION PHASE (Bottom-up Upward Pass) ---
            let (target_kind, start_idx, end_idx) = {
                let node_ref = builder.fetch(current_id)?;
                let children = node_ref.children();

                // Reserve space in the scratchpad buffer to avoid reallocations
                let start = scratch.children_buffer_mut().len();
                scratch.children_buffer_mut().reserve(children.len());

                // Fetch the already-normalized child IDs from the scratchpad cache
                for &old_child_id in children {
                    let new_child_id = scratch
                        .get(old_child_id)
                        .ok_or_else(|| NormalizationError::missing_cache(old_child_id))?;
                    scratch.children_buffer_mut().push(new_child_id);
                }
                let end = scratch.children_buffer_mut().len();

                // Unify and normalize type signatures within quantifier scopes
                let kind = match node_ref.kind().clone() {
                    ExprEntryKind::Forall(mut vars) => {
                        for var in vars.iter_mut() {
                            typed_symbol::normalize_typed_variable(var, registry)?;
                        }
                        ExprEntryKind::Forall(vars)
                    }
                    ExprEntryKind::Exists(mut vars) => {
                        for var in vars.iter_mut() {
                            typed_symbol::normalize_typed_variable(var, registry)?;
                        }
                        ExprEntryKind::Exists(vars)
                    }
                    other_kind => other_kind,
                };

                (kind, start, end)
            }; // <- The immutable borrow on `builder` is released here

            // Extract the newly mapped children slice and reconstruct the node
            let new_children_slice = &scratch.children_buffer()[start_idx..end_idx];
            let new_id = builder.reconstruct(target_kind, new_children_slice)?;

            // Clean up the temporary segment in the scratchpad and cache the result
            scratch.children_buffer_mut().truncate(start_idx);
            scratch.insert(current_id, new_id);
        } else {
            // --- DESCENT PHASE (Discovery Downward Pass) ---
            if scratch.is_visited(current_id) {
                continue;
            }
            scratch.mark_visited(current_id);

            // 1. Re-push the current node as 'processed = true' for the upcoming upward pass
            scratch.push(current_id, true);

            // 2. Extract child IDs into the scratchpad's contiguous buffer
            let (start_idx, end_idx) = {
                let node_ref = builder.fetch(current_id)?;
                let children = node_ref.children();

                let start = scratch.children_buffer_mut().len();
                scratch.children_buffer_mut().extend_from_slice(children);
                let end = scratch.children_buffer_mut().len();

                (start, end)
            };

            // 3. Push children to the stack in reverse order to preserve evaluation sequence
            // (avoids long-term borrows on the scratchpad structure)
            for i in (start_idx..end_idx).rev() {
                let child_id = scratch.children_buffer()[i];
                scratch.push(child_id, false);
            }

            // Clear the temporary child segment from the scratchpad
            scratch.children_buffer_mut().truncate(start_idx);
        }
    }

    // Safely retrieve the final normalized root ID from the cache
    let new_root = scratch
        .get(root)
        .ok_or_else(|| NormalizationError::missing_cache(root))?;

    Ok(new_root)
}
