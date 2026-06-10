//! # HTN and Task Hierarchies Module
//!
//! This module provides specialized constructors for Hierarchical Task Networks (HTN).
//! It handles task symbols, labeling for task decomposition, and ordering constraints.
//!
//! Performance is optimized through the use of `SmallVec` to avoid heap allocations
//! for common task signatures and aggressive inlining of symbol constructors.

use crate::aiplan4rust::lir::expr::ExprBuilder;
use crate::aiplan4rust::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::support::lang::{
    CompareOp, TaskLabelSymbolId, TaskSkeletonId, TaskSymbolId,
};

impl<'a> ExprBuilder<'a> {
    /// Creates a leaf node representing a Task Symbol in an HTN domain.
    ///
    /// # Arguments
    /// * `id` - Any type that can be converted into a `TaskSymbolId` (e.g., raw index or wrapped ID).
    ///
    /// # Returns
    /// An `ExprId` pointing to the unique interned `TaskSymbol` node.
    ///
    /// # Performance
    /// Marked `#[inline(always)]`. As a leaf node with no children, it bypasses
    /// structural hashing, providing near-instantaneous interning.
    #[inline(always)]
    pub fn task_symbol<I: Into<TaskSymbolId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprKind::TaskSymbol(id.into()), &[])
    }

    /// Creates a leaf node representing a Task Label, used to reference specific task
    /// instances within HTN method decompositions.
    ///
    /// # Arguments
    /// * `id` - Any type convertible into a `TaskLabelSymbolId`.
    ///
    /// # Returns
    /// An `ExprId` representing the unique task label node.
    #[inline(always)]
    pub fn task_label<I: Into<TaskLabelSymbolId>>(&mut self, id: I) -> ExprId {
        self.intern(ExprKind::TaskLabel(id.into()), &[])
    }

    /// Associates a unique label with a task expression to create a `LabeledTask`.
    ///
    /// # Arguments
    /// * `id` - The identifier for the task label.
    /// * `task_expr` - The `ExprId` of the actual task being labeled.
    ///
    /// # Returns
    /// An `ExprId` for the labeled task node, linking the label and the task expression.
    #[inline]
    pub fn labeled_task<I: Into<TaskLabelSymbolId>>(&mut self, id: I, task_expr: ExprId) -> ExprId {
        let label_node = self.task_label(id);
        self.intern(ExprKind::LabeledTask, &[label_node, task_expr])
    }

    /// Creates a temporal ordering constraint between two tasks.
    ///
    /// # Arguments
    /// * `task1` - The `ExprId` of the predecessor task.
    /// * `task2` - The `ExprId` of the successor task.
    ///
    /// # Returns
    /// An `ExprId` representing the constraint `task1 < task2`.
    ///
    /// # Note
    /// Semantically, this leverages `CompareOp::Less` to enforce that `task1`
    /// must be completed before `task2` begins.
    #[inline]
    pub fn task_ordering_constraint(&mut self, task1: ExprId, task2: ExprId) -> ExprId {
        self.intern(
            ExprKind::TaskOrderingConstraint(CompareOp::Less),
            &[task1, task2],
        )
    }

    /// Constructs a full Task instance with a skeleton and multiple arguments.
    ///
    /// # Arguments
    /// * `sym_id` - The identifier for the task symbol (the "name" of the task).
    /// * `args` - A slice of `ExprId`s representing the task's parameters.
    /// * `skel_id` - The skeleton identifier defining the task's structural metadata.
    ///
    /// # Returns
    /// An `ExprId` pointing to the interned `Task` node containing the symbol and its arguments.
    ///
    /// # Performance
    /// Uses a **zero-allocation** strategy by swapping the internal `primary_buffer`.
    /// This avoids heap churn and keeps the memory footprint stable even during
    /// massive HTN decomposition phases.
    #[inline]
    pub fn task_with_skeleton<TID, SID>(
        &mut self,
        sym_id: TID,
        args: &[ExprId],
        skel_id: SID,
    ) -> ExprId
    where
        TID: Into<TaskSymbolId>,
        SID: Into<TaskSkeletonId>,
    {
        let sym_node = self.task_symbol(sym_id);
        let skel = skel_id.into();

        let mut buffer = std::mem::take(&mut self.primary_buffer);

        buffer.clear();
        if buffer.capacity() < args.len() + 1 {
            buffer.reserve(args.len() + 1);
        }

        buffer.push(sym_node);
        buffer.extend_from_slice(args);

        let id = self.intern(ExprKind::Task(skel), &buffer);

        self.primary_buffer = buffer;

        id
    }
}
#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprStore};

    /// Verifies HTN specific interning integrity and deduplication.
    #[test]
    fn test_htn_symbols_and_ordering() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Symbol Deduplication
        let s1 = builder.task_symbol(1);
        let s2 = builder.task_symbol(1);
        assert_eq!(s1, s2, "Task symbols must be shared");

        // 2. Labeling Integrity
        let task = builder.task_symbol(10);
        let l1 = builder.labeled_task(1, task);
        let l2 = builder.labeled_task(1, task);
        assert_eq!(
            l1, l2,
            "Labeled tasks must be identical for same label/task pair"
        );

        // 3. Ordering Consistency
        let o1 = builder.task_ordering_constraint(s1, task);
        let o2 = builder.task_ordering_constraint(s1, task);
        assert_eq!(o1, o2, "Ordering constraints must be canonicalized");
    }
    /// Verifies task instantiation with arguments and skeleton integrity.
    #[test]
    fn test_task_with_skeleton_and_arguments() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let arg1 = builder.object(1);
        let arg2 = builder.variable(2);
        let sym_id = 10;
        let skel_id = 100;

        // 1. Check basic instantiation and deduplication
        let t1 = builder.task_with_skeleton(sym_id, &[arg1, arg2], skel_id);
        let t2 = builder.task_with_skeleton(sym_id, &[arg1, arg2], skel_id);

        assert_eq!(t1, t2, "Identical tasks must be hash-consed to the same ID");

        // 2. Resolve the expected task symbol ID BEFORE borrowing the node
        // This avoids the mutable/immutable conflict.
        let expected_sym_node = builder.task_symbol(sym_id);

        // 3. Verify structure via children retrieval
        let node = builder.get(t1).expect("Task node should exist");
        let children = node.children();

        assert_eq!(
            children.len(),
            3,
            "Task should have 3 children (Symbol + 2 Args)"
        );

        // Now we can use children[0] and our pre-resolved ID
        assert_eq!(
            children[0], expected_sym_node,
            "First child must be the task symbol"
        );
        assert_eq!(children[1], arg1);
        assert_eq!(children[2], arg2);
    }
}
