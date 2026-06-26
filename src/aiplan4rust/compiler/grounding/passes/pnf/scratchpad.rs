use crate::aiplan4rust::compiler::lir::expr::ExprId;

/// A reusable, allocation-free scratchpad for PNF/NNF transformations using dual flat lookup tables.
///
/// `PnfScratchpad` serves as a memory buffer to perform iterative, bottom-up tree walks and
/// hash-consed reconstructions of expression trees. By mirroring the linear layout of the
/// `ExprStore` via dual flat `Vec` arenas, it eliminates both dynamic heap allocations and
/// hashing overhead on the critical path.
///
/// # Layout and Optimizations
///
/// * **Stack-driven traversal**: Replaces recursive execution with an explicit data stack to guarantee
///   immunity against stack overflows on deeply nested syntax trees.
/// * **Flat O(1) Array Indexing**: Replaces `FxHashMap` lookup with direct array indexing (`cache[id]`),
///   leveraging the fact that `ExprId` matches sequential arena indices to achieve maximum CPU cache locality.
/// * **Lazy buffer caching**: Contains pre-allocated vector fields designed for scratch operations,
///   such as structural node tracking and child slice updates.
pub struct PnfScratchpad {
    /// Explicit execution stack tracking tuples of `(old_expr_id, in_condition, children_pushed)`.
    pub(crate) stack: Vec<(ExprId, bool, bool)>,
    /// Memoization cache for downward conditional context (`in_condition = true`).
    pub(crate) cache_true: Vec<ExprId>,
    /// Memoization cache for downward effect context (`in_condition = false`).
    pub(crate) cache_false: Vec<ExprId>,
    /// Flattened reusable buffer for assembling and modifying child node pointers before interning.
    pub(crate) children_buffer: Vec<ExprId>,
}

impl PnfScratchpad {
    /// Initial capacity for the explicit non-recursive DFS traversal stack.
    const STACK_CAPACITY: usize = 32;

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
            cache_true: Vec::new(),
            cache_false: Vec::new(),
            children_buffer: Vec::with_capacity(Self::CHILDREN_BUFFER_CAPACITY),
        }
    }

    /// Clears all internal buffers and resizes the dual cache maps to match the current `ExprStore` topology.
    ///
    /// The memory contents are zeroed/reset using highly-optimized SIMD-vectorized `.fill()` blocks,
    /// ensuring zero dynamic allocations once the underlying capacities stabilize.
    ///
    /// # Parameters
    ///
    /// * `store_len` - The total number of unique registered expressions currently sitting in the `ExprStore`.
    pub fn clear(&mut self, store_len: usize) {
        self.stack.clear();
        self.children_buffer.clear();

        // ExprId::default() acts as our ExprId::NONE sentinel
        let default_id = ExprId::default();

        // Resize the dual vector arenas if the store has grown (rarely triggers reallocs after warmup)
        self.cache_true.resize(store_len, default_id);
        self.cache_false.resize(store_len, default_id);

        // Blazing-fast sequential memory sweep (compiles down to an optimized memset/SIMD loop)
        self.cache_true.fill(default_id);
        self.cache_false.fill(default_id);
    }
}
