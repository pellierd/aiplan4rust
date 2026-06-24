use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::support::lang::{typed_list, TypeId, TypedList, VariableId};
use fxhash::FxHashMap;

/// A high-performance, reusable memory arena designed to eliminate
/// runtime heap allocations during quantifier expansion and expression tree traversals.
///
/// By acting as a stateful scratchpad, it hosts the stack for non-recursive post-order
/// traversals, a memoization cache for sub-expression sharing, and flat buffers
/// for dynamic expression interning.
///
/// # Arguments
/// * `cache` - A high-speed memoization map translating original ungrounded `ExprId`s to their expanded equivalents.
/// * `children_buffer` - A flat, dynamic buffer used to aggregate transformed child IDs before re-interning operations.
/// * `stack` - An explicit execution stack supporting non-recursive DFS post-order traversals. The boolean flag marks whether a node's children have already been discovered and pushed.
/// * `variables` - A pre-allocated list holding the scope-specific variables of the quantifier being processed.
/// * `instances_buffer` - A reusable accumulation vector that collects sub-trees generated during the Cartesian product expansion of a quantifier, preventing vector allocations per iteration.
#[derive(Debug)]
pub struct QnfScratchpad {
    pub(crate) cache: FxHashMap<ExprId, ExprId>,
    pub(crate) children_buffer: Vec<ExprId>,
    pub(crate) stack: Vec<(ExprId, bool)>,
    pub(crate) variables: TypedList<VariableId, TypeId>,
    pub(crate) instances_buffer: Vec<ExprId>,
}

impl QnfScratchpad {
    /// Initial capacity for the expression memoization cache.
    const CACHE_CAPACITY: usize = 32;

    /// Initial capacity for the child expression accumulation buffer.
    const CHILDREN_BUFFER_CAPACITY: usize = 8;

    /// Initial capacity for the explicit non-recursive DFS post-order traversal stack.
    const STACK_CAPACITY: usize = 32;

    /// Initial capacity for the quantifier instance expansion buffer.
    const INSTANCES_BUFFER_CAPACITY: usize = 64;

    /// Constructs a new `QnfScratchpad` with highly optimized initial inline capacities
    /// tailored to minimize early structural resizing.
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::with_capacity_and_hasher(Self::CACHE_CAPACITY, Default::default()),
            children_buffer: Vec::with_capacity(Self::CHILDREN_BUFFER_CAPACITY),
            stack: Vec::with_capacity(Self::STACK_CAPACITY),
            variables: TypedList::with_capacity(typed_list::OPTIMAL_LIST_CAPACITY),
            instances_buffer: Vec::with_capacity(Self::INSTANCES_BUFFER_CAPACITY),
        }
    }

    /// Clears all internal collections, resetting the scratchpad state for the next
    /// expansion pass while retaining the underlying heap allocations for performance.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.children_buffer.clear();
        self.stack.clear();
        self.variables.clear();
        // Clear the buffer to ensure isolation between separate quantifier expansion passes
        self.instances_buffer.clear();
    }
}
