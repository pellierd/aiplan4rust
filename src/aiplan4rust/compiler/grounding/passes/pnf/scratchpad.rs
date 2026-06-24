use crate::aiplan4rust::compiler::lir::expr::ExprId;
use fxhash::FxHashMap;

/// A reusable, allocation-free scratchpad for PNF (Positive Normal Form) transformations.
///
/// `PnfScratchpad` serves as a memory buffer to perform iterative, bottom-up tree walks and
/// hash-consed reconstructions of expression trees. By retaining its allocated capacities
/// across multiple compiler passes via the [`clear`](Self::clear) method, it completely
/// eliminates dynamic heap allocations on the critical path.
///
/// # Layout and Optimizations
///
/// * **Stack-driven traversal**: Replaces recursive execution with an explicit data stack to guarantee
///   immunity against stack overflows on deeply nested syntax trees.
/// * **Fast Integer Hashing**: Employs `FxHashMap` (a non-cryptographic, high-performance hasher)
///   optimized specifically for primitive integer-like keys such as [`ExprId`].
/// * **Lazy buffer caching**: Contains pre-allocated vector fields designed for scratch operations,
///   such as structural node tracking and child slice updates.
#[derive(Default)]
pub struct PnfScratchpad {
    /// Explicit execution stack tracking tuples of `(old_expr_id, in_condition, children_pushed)`.
    pub(crate) stack: Vec<(ExprId, bool, bool)>,
    /// Memoization cache mapping original expression IDs to their restructured PNF expression IDs.
    pub(crate) cache: FxHashMap<ExprId, ExprId>,
    /// Flattened reusable buffer for assembling and modifying child node pointers before interning.
    pub(crate) children_buffer: Vec<ExprId>,
}

impl PnfScratchpad {
    /// Initial capacity for the explicit non-recursive DFS traversal stack.
    const STACK_CAPACITY: usize = 32;

    /// Initial capacity for the expression PNF restructuring memoization cache.
    const CACHE_CAPACITY: usize = 64;

    /// Initial capacity for the child expression accumulation buffer.
    const CHILDREN_BUFFER_CAPACITY: usize = 8;

    /// Creates a new `PnfScratchpad` initialized with conservative default capacities
    /// to minimize initial reallocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
    /// let mut scratchpad = PnfScratchpad::new();
    /// ```
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(Self::STACK_CAPACITY),
            cache: FxHashMap::with_capacity_and_hasher(Self::CACHE_CAPACITY, Default::default()),
            children_buffer: Vec::with_capacity(Self::CHILDREN_BUFFER_CAPACITY),
        }
    }

    /// Clears all internal buffers, resetting their logical lengths to zero while
    /// completely retaining the underlying heap-allocated capacities.
    ///
    /// This method must be called at the entry point of every independent PNF lowering pass
    /// to avoid state leakage and preserve optimal throughput.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.cache.clear(); // Recycles allocated buckets without triggering drop/realloc overhead
        self.children_buffer.clear();
    }
}
