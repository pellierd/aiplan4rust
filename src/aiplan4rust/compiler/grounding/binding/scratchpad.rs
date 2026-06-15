use crate::aiplan4rust::compiler::lir::expr::ExprId;
use fxhash::FxHashMap;

/// ### Binding Scratchpad
///
/// A stateful memory scratchpad dedicated to variable substitution (`binding`) operations.
///
/// This structural component adheres to a **zero-hot-allocation design pattern**, serving
/// as a reusable, long-lived buffer container. By maintaining internal pre-allocated buffers
/// across multiple binding executions, it drastically reduces heap pressure and memory
/// fragmentation during high-frequency PDDL grounding phases.
pub struct BindingScratchpad {
    /// Internal translation cache mapping original `ExprId` keys to their updated,
    /// substituted, or statically pruned `ExprId` values.
    /// Crucial for preventing redundant traversals over shared Directed Acyclic Graph (DAG) branches.
    pub(in crate::aiplan4rust) substitution_map: FxHashMap<ExprId, ExprId>,

    /// Reusable continuous buffer used to transiently aggregate the transformed child
    /// identifiers of a compound node before routing them into the smart constructor pipeline.
    pub(in crate::aiplan4rust) children_buffer: Vec<ExprId>,

    /// Explicit work stack backing the manual, non-recursive iterative post-order traversal loop.
    /// Tracks tuples of `(ExprId, children_pushed)` to mimic call stack frames safely on the heap.
    pub(in crate::aiplan4rust) stack: Vec<(ExprId, bool)>,
}

impl BindingScratchpad {
    /// Creates a new `BindingScratchpad` instance equipped with targeted initial heap capacities.
    ///
    /// # Returns
    /// A clean `BindingScratchpad` with pre-allocated internal maps and vectors to prevent
    /// immediate resizing overhead during routine expression evaluations.
    pub fn new() -> Self {
        Self {
            substitution_map: FxHashMap::with_capacity_and_hasher(32, Default::default()),
            children_buffer: Vec::with_capacity(8),
            stack: Vec::with_capacity(32),
        }
    }

    /// Resets all underlying buffers for immediate reuse without dropping their allocated capacities.
    ///
    /// This method performs an in-place clearing operation, stripping the metadata while
    /// keeping the memory blocks warm and ready for the next binding iteration.
    pub fn clear(&mut self) {
        self.substitution_map.clear();
        self.children_buffer.clear();
        self.stack.clear();
    }
}
