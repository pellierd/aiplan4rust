//! Datalog Compilation Scratchpad Module.
//!
//! This module provides the `DatalogScratchpad` structure, which serves as a reusable
//! temporary workspace to eliminate heap allocations and buffer reallocations during
//! the deep graph traversals and expression processing phases of the Datalog pipeline.

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::support::lang::AtomSkeletonId;
use ahash::HashSetExt;
use rustc_hash::FxHashSet;

/// Temporary storage structure designed to prevent heap allocations during Datalog encoding.
///
/// `DatalogScratchpad` pre-allocates reusable buffers and stacks, allowing recursive or iterative
/// traversal algorithms (such as condition and effect analyses) to execute with zero runtime allocation overhead.
pub struct DatalogScratchpad {
    /// Buffer tracking visited nodes during expression graph traversals.
    pub(crate) visited: Vec<bool>,
    /// Generic stack utilized for simple traversal passes (e.g., alias resolution).
    pub(crate) stack: Vec<ExprId>,
    /// Stack tracking condition expression traversals using a post-order state (`ExprId` + visited flag).
    pub(crate) condition_stack: Vec<(ExprId, bool)>,
    /// Stack tracking effect traversals pairing expression identifiers with their active cause atoms.
    pub(crate) effect_stack: Vec<(ExprId, Atom)>,
    /// Tracking set for processed `(expression, cause)` pairs to avoid allocations in effect encoding.
    pub(crate) visited_effects: FxHashSet<(ExprId, AtomSkeletonId)>,
}

impl DatalogScratchpad {
    /// Creates a new `DatalogScratchpad` with an initial estimated capacity to minimize future reallocations.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The preliminary capacity allocated for the `visited` node buffer.
    ///
    /// # Returns
    ///
    /// Returns a pre-initialized scratchpad instance ready for execution passes.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            visited: Vec::with_capacity(capacity),
            stack: Vec::with_capacity(64),
            condition_stack: Vec::with_capacity(64),
            effect_stack: Vec::with_capacity(64),
            visited_effects: FxHashSet::with_capacity(64),
        }
    }

    /// Prepares and resets the `visited` buffer for a specific expression store size.
    ///
    /// # Arguments
    ///
    /// * `size` - The required element count, matching the store length.
    #[inline]
    pub fn prepare_visited(&mut self, size: usize) {
        self.visited.resize(size, false);
        self.visited.fill(false);
    }

    /// Clears and initializes the generic stack with a new root node for traversal.
    ///
    /// # Arguments
    ///
    /// * `root` - The starting expression node identifier.
    #[inline]
    pub fn prepare_stack(&mut self, root: ExprId) {
        self.stack.clear();
        self.stack.push(root);
    }

    /// Clears and initializes the condition stack with a root node for post-order evaluation.
    ///
    /// # Arguments
    ///
    /// * `root` - The starting condition expression identifier.
    #[inline]
    pub fn prepare_condition_stack(&mut self, root: ExprId) {
        self.condition_stack.clear();
        self.condition_stack.push((root, false));
    }

    /// Clears and initializes the effect stack with a root expression and its originating cause atom.
    ///
    /// # Arguments
    ///
    /// * `root` - The starting effect expression identifier.
    /// * `root_cause` - The triggering atom associated with the effect.
    #[inline]
    pub fn prepare_effect_stack(&mut self, root: ExprId, root_cause: Atom) {
        self.effect_stack.clear();
        self.effect_stack.push((root, root_cause));
    }

    /// Clears the visited effects hash set in preparation for a new effect analysis pass.
    #[inline]
    pub fn prepare_effects_visited(&mut self) {
        self.visited_effects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::reachability::datalog::core::atom::AtomArgs;

    /// # Objective
    /// Validates that a newly created `DatalogScratchpad` initialized with a specific capacity
    /// starts with empty internal buffers and collections.
    ///
    /// # Input
    /// - Initial capacity: `32` via `DatalogScratchpad::with_capacity(32)`
    ///
    /// # Expected Output
    /// - All internal vectors (`visited`, `stack`, `condition_stack`, `effect_stack`) and hash sets (`visited_effects`) are empty.
    #[test]
    fn test_scratchpad_capacity_initialization() {
        let scratchpad = DatalogScratchpad::with_capacity(32);
        assert!(scratchpad.visited.is_empty());
        assert!(scratchpad.stack.is_empty());
        assert!(scratchpad.condition_stack.is_empty());
        assert!(scratchpad.effect_stack.is_empty());
        assert!(scratchpad.visited_effects.is_empty());
    }

    /// # Objective
    /// Tests that the `prepare_visited` method correctly resizes the visited buffer
    /// and ensures all elements are set to `false`.
    ///
    /// # Input
    /// - Initial buffer size request: `5`
    /// - Subsequent modification: manually setting an element to `true`, then resizing to `4`.
    ///
    /// # Expected Output
    /// - Buffer length matches the requested size.
    /// - All values in the buffer are set to `false` after preparation.
    #[test]
    fn test_prepare_visited_buffer() {
        let mut scratchpad = DatalogScratchpad::with_capacity(10);
        scratchpad.prepare_visited(5);

        assert_eq!(scratchpad.visited.len(), 5);
        assert!(scratchpad.visited.iter().all(|&v| !v));

        // Test resizing and re-filling with false
        scratchpad.visited[2] = true;
        scratchpad.prepare_visited(4);
        assert_eq!(scratchpad.visited.len(), 4);
        assert!(scratchpad.visited.iter().all(|&v| !v));
    }

    /// # Objective
    /// Verifies that generic and condition stacks are properly cleared and seeded
    /// with initial root expression nodes.
    ///
    /// # Input
    /// - Root expression ID: `ExprId::from(42)`
    ///
    /// # Expected Output
    /// - The generic stack contains exactly 1 element equal to the root expression.
    /// - The condition stack contains exactly 1 element matching the root expression combined with a `false` visited flag.
    #[test]
    fn test_stack_preparation_methods() {
        let mut scratchpad = DatalogScratchpad::with_capacity(10);
        let root_expr = ExprId::from(42);

        // Test generic stack
        scratchpad.prepare_stack(root_expr);
        assert_eq!(scratchpad.stack.len(), 1);
        assert_eq!(scratchpad.stack[0], root_expr);

        // Test condition stack
        scratchpad.prepare_condition_stack(root_expr);
        assert_eq!(scratchpad.condition_stack.len(), 1);
        assert_eq!(scratchpad.condition_stack[0], (root_expr, false));
    }

    /// # Objective
    /// Ensures that the effects tracking reset mechanism successfully executes without errors.
    ///
    /// # Input
    /// - A scratchpad instance with default or empty state.
    ///
    /// # Expected Output
    /// - `visited_effects` remains empty after calling `prepare_effects_visited`.
    #[test]
    fn test_effects_tracking_reset() {
        let mut scratchpad = DatalogScratchpad::with_capacity(10);

        // Simulate an addition in visited_effects (if types allow in test context)
        // Or simply verify that clear works on an empty/filled set
        scratchpad.prepare_effects_visited();
        assert!(scratchpad.visited_effects.is_empty());
    }

    /// # Objective
    /// Verifies that clearing operations on internal stacks preserve their pre-allocated capacity
    /// to maintain high performance and prevent unnecessary reallocations.
    ///
    /// # Input
    /// - Initial scratchpad capacity: `64`
    /// - Push operation followed by a clear operation on the generic stack.
    ///
    /// # Expected Output
    /// - Stack capacity remains greater than or equal to the initial pre-allocated capacity.
    #[test]
    fn test_capacity_retention_and_effect_stack() {
        let mut scratchpad = DatalogScratchpad::with_capacity(64);

        // Ensure clearing operations preserve pre-allocated capacity for performance
        let initial_capacity = scratchpad.stack.capacity();
        scratchpad.prepare_stack(ExprId::from(10));
        scratchpad.stack.clear();
        assert!(scratchpad.stack.capacity() >= initial_capacity);
    }

    /// # Objective
    /// Validates that `prepare_effect_stack` correctly initializes the effect stack
    /// with an expression identifier and an associated cause atom.
    ///
    /// # Input
    /// - Root expression ID: `ExprId::from(10)`
    /// - Dummy atom generated via `Atom::nary`
    ///
    /// # Expected Output
    /// - Effect stack length is 1, containing the expected expression ID and atom pair.
    #[test]
    fn test_prepare_effect_stack() {
        let mut scratchpad = DatalogScratchpad::with_capacity(10);
        let root_expr = ExprId::from(10);
        let dummy_atom = Atom::nary(AtomSkeletonId::from(1), AtomArgs::new());

        scratchpad.prepare_effect_stack(root_expr, dummy_atom);
        assert_eq!(scratchpad.effect_stack.len(), 1);
        assert_eq!(scratchpad.effect_stack[0].0, root_expr);
    }

    /// # Objective
    /// Confirms that `prepare_effects_visited` successfully clears tracked expression-cause pairs
    /// from the `visited_effects` set.
    ///
    /// # Input
    /// - A dummy `(ExprId, AtomSkeletonId)` entry manually inserted into `visited_effects`.
    ///
    /// # Expected Output
    /// - `visited_effects` is non-empty before reset, and empty after calling `prepare_effects_visited`.
    #[test]
    fn test_visited_effects_clear() {
        let mut scratchpad = DatalogScratchpad::with_capacity(10);

        // Insert a dummy pair to verify actual clearing functionality
        scratchpad
            .visited_effects
            .insert((ExprId::from(1), AtomSkeletonId::from(1)));
        assert!(!scratchpad.visited_effects.is_empty());

        scratchpad.prepare_effects_visited();
        assert!(scratchpad.visited_effects.is_empty());
    }
}
