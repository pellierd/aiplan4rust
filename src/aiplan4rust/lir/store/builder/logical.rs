//! This module provides a high-performance, zero-allocation expression builder
//! for the Lower-level Intermediate Representation (LIR).
//!
//! The [`ExprBuilder`] uses techniques such as Hash-Consing, canonicalization,
//! and advanced conditional effect merging to ensure that logical expressions
//! are simplified and unique within the [`ExprStore`].

use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Creates a logical `AND` node with one or more children.
    ///
    /// This is the high-level entry point for conjunctions. It delegates to `reduce`,
    /// which automatically handles flattening, sorting, deduplication, and
    /// conditional effect merging to ensure the resulting `ExprId` is canonical.
    ///
    /// # Arguments
    /// * `children` - A slice of `ExprId` operands to be joined by an AND operator.
    ///
    /// # Returns
    /// The unique `ExprId` representing the simplified conjunction.
    pub fn and(&mut self, children: &[ExprId]) -> ExprId {
        self.reduce(ExprEntryKind::And, children)
    }

    /// Creates an empty `AND` node, representing the logical constant "True".
    ///
    /// In mathematical logic, the identity element for the AND operation
    /// (the empty conjunction) is defined as **True**.
    ///
    /// # Returns
    /// The canonical `ExprId` for the constant "True".
    pub fn empty_and(&mut self) -> ExprId {
        self.intern(ExprEntryKind::And, &[])
    }

    /// Creates a logical `OR` node with one or more children.
    ///
    /// This is the high-level entry point for disjunctions. It delegates to `reduce`,
    /// which simplifies the operands (including tautology detection) to return
    /// a canonical `ExprId`.
    ///
    /// # Arguments
    /// * `children` - A slice of `ExprId` operands to be joined by an OR operator.
    ///
    /// # Returns
    /// The unique `ExprId` representing the simplified disjunction.
    pub fn or(&mut self, children: &[ExprId]) -> ExprId {
        self.reduce(ExprEntryKind::Or, children)
    }

    /// Creates an empty `OR` node, representing the logical constant "False".
    ///
    /// In mathematical logic, the identity element for the OR operation
    /// (the empty disjunction) is defined as **False**.
    ///
    /// # Returns
    /// The canonical `ExprId` for the constant "False".
    pub fn empty_or(&mut self) -> ExprId {
        self.intern(ExprEntryKind::Or, &[])
    }

    /// Reduces a logical expression by applying flattening, constant folding,
    /// and canonicalization rules.
    ///
    /// This "Smart Constructor" ensures that all `AND` and `OR` expressions are
    /// simplified on the fly before being interned. It maintains the Low-level
    /// Intermediate Representation (LIR) in a highly optimized state.
    ///
    /// # Logical Transformations
    /// 1. **Absorbing Element**: Returns `False` for `AND(..., False)` or `True` for `OR(..., True)`.
    /// 2. **Neutral Element**: Removes `True` from `AND` or `False` from `OR`.
    /// 3. **Flattening**: Merges nested operations of the same kind, e.g., `AND(A, AND(B, C))` -> `AND(A, B, C)`.
    /// 4. **Canonicalization**: Sorts and deduplicates operands to ensure a unique representation.
    /// 5. **Complementary Pairs**: Detects tautologies (`P OR !P`) and contradictions (`P AND !P`).
    /// 6. **Conditional Fusion**: Triggers advanced merging for `When` nodes within `AND` blocks.
    ///
    /// # Arguments
    /// * `kind` - The type of logical operation (`And` or `Or`).
    /// * `children` - The raw list of operand `ExprId`s.
    ///
    /// # Returns
    /// A canonical `ExprId` representing the simplified expression.
    ///
    /// # Implementation Details
    /// To bypass the Rust borrow checker and maintain zero-allocation, this function
    /// leverages disjoint field access (`self.store` vs `self.primary_buffer`) and
    /// uses `secondary_buffer` as a temporary staging area for flattening.
    pub fn reduce(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
        let (neutral, absorbing) = match kind {
            ExprEntryKind::And => (self.empty_and(), self.empty_or()),
            ExprEntryKind::Or => (self.empty_or(), self.empty_and()),
            _ => return self.intern(kind, children),
        };

        self.primary_buffer.clear();
        let mut has_when = false;

        // --- STEP 1: Collection & Flattening ---
        for &child in children {
            if child == absorbing {
                return absorbing;
            }
            if child == neutral {
                continue;
            }

            // Access store directly to allow simultaneous mutable access to buffers
            if let Some(node) = self.store.get(child) {
                if node.kind() == &kind {
                    // Stage children in secondary_buffer to avoid double-borrowing self
                    self.secondary_buffer.clear();
                    self.secondary_buffer.extend_from_slice(node.children());
                    self.primary_buffer
                        .extend_from_slice(&self.secondary_buffer);
                    continue;
                } else if kind == ExprEntryKind::And && node.kind() == &ExprEntryKind::When {
                    has_when = true;
                }
            }
            self.primary_buffer.push(child);
        }

        // --- STEP 2: Sorting & Deduplication ---
        let len = self.primary_buffer.len();
        if len == 0 {
            return neutral;
        }
        if len == 1 {
            return self.primary_buffer[0];
        }

        self.sort_buffer_by_id(len);
        self.primary_buffer.dedup();

        // --- STEP 3: Final Analysis & Interning ---
        // Ownership dance: Move buffer out to perform analysis, then return it.
        let mut collected = std::mem::take(&mut self.primary_buffer);

        let result = if self.has_complementary_pair(&collected) {
            absorbing
        } else {
            match kind {
                ExprEntryKind::And if has_when => self.finalize_and_with_merge(&collected),
                _ => self.intern(kind, &collected),
            }
        };

        // Restore buffer capacity for future use
        collected.clear();
        self.primary_buffer = collected;
        result
    }

    /// Sorts the entire primary buffer by `ExprId` to enable deduplication and canonicalization.
    ///
    /// This is a specialized implementation of the Heapsort algorithm. Sorting operands
    /// by their unique internal identifiers ensures that logical expressions are stored
    /// in a consistent (canonical) order, which is a prerequisite for efficient
    /// Hash-Consing.
    ///
    /// # Arguments
    /// * `len` - The number of elements in `primary_buffer` to sort.
    ///
    /// # Complexity
    /// - **Time**: O(n log n) in all cases.
    /// - **Space**: O(1), as the sort is performed in-place.
    fn sort_buffer_by_id(&mut self, len: usize) {
        // Phase 1: Build a max-heap from the IDs
        for start in (0..len / 2).rev() {
            self.sift_down_by_id(start, len);
        }

        // Phase 2: Extract elements from the heap to build the sorted array
        for end in (1..len).rev() {
            // Swap the current maximum (at index 0) with the last unsorted element
            self.primary_buffer.swap(0, end);
            // Restore the heap property for the remaining unsorted portion
            self.sift_down_by_id(0, end);
        }
    }

    /// Restores the max-heap property for the buffer based on raw `ExprId` values.
    ///
    /// This helper pushes a value down the binary tree until it is greater than
    /// or equal to its children. It uses raw ID comparisons, making it significantly
    /// faster than sorting by complex node properties.
    ///
    /// # Arguments
    /// * `root` - The index of the element to start sifting down.
    /// * `end` - The upper bound of the current heap segment.
    fn sift_down_by_id(&mut self, mut root: usize, end: usize) {
        while root * 2 + 1 < end {
            let mut child = root * 2 + 1; // Left child index

            // If right child exists and is greater than left child, move to right child
            if child + 1 < end && self.primary_buffer[child] < self.primary_buffer[child + 1] {
                child += 1;
            }

            // If the root is smaller than the largest child, swap and continue sifting
            if self.primary_buffer[root] < self.primary_buffer[child] {
                self.primary_buffer.swap(root, child);
                root = child;
            } else {
                // Heap property is satisfied
                break;
            }
        }
    }

    /// Orchestrates the advanced simplification of an `AND` node by merging compatible `When` effects.
    ///
    /// This function coordinates a multi-step pipeline to reduce the complexity of conditional
    /// effects within a logical `AND` block. It specifically targets the fusion of
    /// `(when C1 E) AND (when C2 E)` into `(when (C1 OR C2) E)`.
    ///
    /// # Pipeline Steps
    /// 1. **Partitioning**: Isolates `When` nodes at the beginning of the buffer.
    /// 2. **Early Exit**: If 0 or 1 `When` node is found, it proceeds to standard internment.
    /// 3. **Sorting**: Orders `When` nodes by their effect ID to group potential merge candidates.
    /// 4. **Merging**: Performs the logical fusion of contiguous nodes sharing the same effect.
    /// 5. **Finalization**: Compacts the buffer to remove gaps and interns the final expression.
    ///
    /// # Arguments
    /// * `children` - The slice of `ExprId` operands to be simplified and merged.
    ///
    /// # Returns
    /// The `ExprId` of the resulting simplified expression (either a merged `When`,
    /// a simplified `AND`, or a single operand).
    ///
    /// # Performance Note
    /// This function is an optimization for PDDL-like structures where many conditional
    /// effects often share the same consequence. It maintains the zero-allocation
    /// guarantee by reusing the builder's internal buffers.
    fn finalize_and_with_merge(&mut self, children: &[ExprId]) -> ExprId {
        // Step 1: Group all 'When' nodes at the front. Returns count of 'When's.
        let split_idx = self.prepare_and_partition(children);

        // Step 2: Optimization - if no potential for merging, intern immediately.
        if split_idx <= 1 {
            return self.intern(ExprEntryKind::And, children);
        }

        // Step 3: Sort 'When's by Effect ID to make duplicates contiguous.
        self.sort_whens_by_effect(split_idx);

        // Step 4: Merge identical effects. Returns new end index of 'When' segment.
        let write_idx = self.merge_consecutive_whens(split_idx);

        // Step 5: Clean up gaps, handle unary reduction, and intern.
        self.finalize_compacted_buffer(split_idx, write_idx)
    }

    /// Initializes the primary buffer and partitions it to move `When` nodes to the front.
    ///
    /// This function acts as the preprocessing step for conditional effect merging.
    /// It populates the `primary_buffer` with the provided children and reorders them
    /// so that all `When` nodes occupy a contiguous prefix of the buffer.
    ///
    /// # Algorithm
    /// It uses a single-pass partitioning logic (similar to the Hoare partition scheme's
    /// focus) to swap `When` nodes into the `[0..split_idx]` range.
    ///
    /// # Arguments
    /// * `children` - The raw slice of `ExprId` operands to be processed.
    ///
    /// # Returns
    /// The `split_idx` (usize), representing the number of `When` nodes found.
    /// These nodes are now located in `primary_buffer[0..split_idx]`, while all other
    /// node types are located in `primary_buffer[split_idx..]`.
    ///
    /// # Complexity
    /// - **Time**: O(n), where n is the number of children.
    /// - **Space**: O(n) for the initial buffer population (amortized zero-alloc).
    fn prepare_and_partition(&mut self, children: &[ExprId]) -> usize {
        self.primary_buffer.clear();
        self.primary_buffer.extend_from_slice(children);

        let mut split_idx = 0;
        for i in 0..self.primary_buffer.len() {
            // Check if the current child is a 'When' node
            if self
                .get(self.primary_buffer[i])
                .map_or(false, |n| n.kind() == &ExprEntryKind::When)
            {
                // Swap the 'When' node to the current split position
                self.primary_buffer.swap(i, split_idx);
                split_idx += 1;
            }
        }
        split_idx
    }

    /// Sorts the `When` nodes in the primary buffer based on their effect IDs.
    ///
    /// This function implements an in-place Heapsort algorithm specifically for the
    /// prefix of the buffer containing conditional effects. By sorting nodes by
    /// their effect ID, it ensures that all `When` nodes targeting the same
    /// consequence are contiguous, facilitating the subsequent merging phase.
    ///
    /// # Algorithm
    /// 1. **Heapify**: Transforms the segment `[0..split_idx]` into a max-heap
    ///    using the `sift_down` logic.
    /// 2. **Sort**: Repeatedly moves the largest element to the end of the
    ///    segment and restores the heap property for the remaining elements.
    ///
    /// # Arguments
    /// * `split_idx` - The number of `When` nodes to sort at the beginning of the buffer.
    ///
    /// # Complexity
    /// - **Time**: O(n log n), where n is `split_idx`.
    /// - **Space**: O(1), as it uses no additional heap memory.
    fn sort_whens_by_effect(&mut self, split_idx: usize) {
        // Phase 1: Build the heap (heapify)
        for start in (0..split_idx / 2).rev() {
            self.sift_down(start, split_idx);
        }

        // Phase 2: Extract elements from the heap to produce a sorted array
        for end in (1..split_idx).rev() {
            // Move current max (root) to the end of the unsorted segment
            self.primary_buffer.swap(0, end);
            // Restore max-heap property for the reduced heap
            self.sift_down(0, end);
        }
    }

    /// Merges consecutive `When` nodes that share the same effect into a single conditional node.
    ///
    /// This function performs a logical fusion: `(when C1 E) AND (when C2 E)` becomes `(when (C1 OR C2) E)`.
    /// This significantly reduces the number of nodes in the store and simplifies downstream processing.
    ///
    /// # Algorithm
    /// 1. **Isolation**: Copies `When` IDs to `secondary_buffer` to free up `primary_buffer`.
    /// 2. **Grouping**: Iterates through the sorted nodes to find contiguous blocks with the same effect.
    /// 3. **Fusion**: For each block, collects conditions into `primary_buffer`, simplifies them via `self.or()`,
    ///    and creates a new unified `When` node.
    /// 4. **Restoration**: Writes the merged nodes back to the start of `primary_buffer`.
    ///
    /// # Reentrancy & Safety
    /// This function is safe for recursive calls. By using `secondary_buffer` for source data,
    /// it ensures that calls to `self.or()` or `self.when()`—which may clear or modify
    /// `primary_buffer`—do not corrupt the iteration state.
    ///
    /// # Arguments
    /// * `split_idx` - The number of `When` nodes located at the beginning of `primary_buffer`.
    ///
    /// # Returns
    /// The new count of `When` nodes after fusion (the `write_idx`).
    fn merge_consecutive_whens(&mut self, split_idx: usize) -> usize {
        if split_idx == 0 {
            return 0;
        }

        // 1. Isolation: Move IDs to secondary_buffer to protect them from recursive builder calls.
        self.secondary_buffer.clear();
        for i in 0..split_idx {
            self.secondary_buffer.push(self.primary_buffer[i]);
        }

        let mut write_idx = 0;
        let mut i = 0;
        let num_whens = self.secondary_buffer.len();

        while i < num_whens {
            let current_when_id = self.secondary_buffer[i];
            let current_eff = self.store.get(current_when_id).unwrap().children()[1];
            let start = i;

            // Group When nodes by identical effect
            while i < num_whens
                && self.store.get(self.secondary_buffer[i]).unwrap().children()[1] == current_eff
            {
                i += 1;
            }

            if i - start > 1 {
                // Collect all conditions for the same effect into the now-free primary_buffer
                self.primary_buffer.clear();
                for j in start..i {
                    self.primary_buffer
                        .push(self.store.get(self.secondary_buffer[j]).unwrap().children()[0]);
                }

                // Temporary take to satisfy borrow checker during recursive OR call
                let tmp_conds = std::mem::take(&mut self.primary_buffer);
                let merged_cond = self.or(&tmp_conds);
                self.primary_buffer = tmp_conds;

                // Create the unified When node
                let new_when = self.when(merged_cond, current_eff);
                self.secondary_buffer[write_idx] = new_when;
            } else {
                // No fusion needed for a single effect
                self.secondary_buffer[write_idx] = self.secondary_buffer[start];
            }
            write_idx += 1;
        }

        // 2. Restoration: Copy results back to primary_buffer
        for j in 0..write_idx {
            self.primary_buffer[j] = self.secondary_buffer[j];
        }

        write_idx
    }

    /// Finalizes the expression buffer after `When` nodes have been merged and compacted.
    ///
    /// This function handles two critical final steps:
    /// 1. **Compaction**: If fusions occurred, it moves non-`When` nodes to the left to
    ///    fill the gap left by merged conditional effects, then truncates the buffer.
    /// 2. **Unary Reduction**: If only one child remains after all simplifications,
    ///    it returns that child directly instead of creating an unnecessary `AND` node.
    ///
    /// Finally, it interns the resulting collection into the `ExprStore`.
    ///
    /// # Arguments
    /// * `split_idx` - The original starting index of non-`When` nodes before merging.
    /// * `write_idx` - The new ending index of the merged `When` nodes.
    ///
    /// # Returns
    /// The `ExprId` of the finalized, simplified expression.
    ///
    /// # Memory
    /// Uses `std::mem::take` and `copy_within` to ensure zero-allocation during
    /// buffer manipulation and internment.
    fn finalize_compacted_buffer(&mut self, split_idx: usize, write_idx: usize) -> ExprId {
        // If write_idx < split_idx, fusions took place; we must shift the remaining nodes
        if write_idx < split_idx {
            let non_whens_len = self.primary_buffer.len() - split_idx;
            if non_whens_len > 0 {
                // Shift non-When nodes to be contiguous with merged When nodes
                self.primary_buffer
                    .copy_within(split_idx..split_idx + non_whens_len, write_idx);
            }
            self.primary_buffer.truncate(write_idx + non_whens_len);
        }

        // Unary reduction: if only one element remains (e.g., a single When or Atom),
        // we avoid creating an AND wrapper.
        if self.primary_buffer.len() == 1 {
            return self.primary_buffer[0];
        }

        // Intern the buffer. We take ownership of the Vec temporarily to satisfy
        // the borrow checker, then return it to primary_buffer to reuse its capacity.
        let mut data = std::mem::take(&mut self.primary_buffer);
        let id = self.intern(ExprEntryKind::And, &data);
        data.clear();
        self.primary_buffer = data;
        id
    }

    /// Restores the max-heap property for the 'When' nodes segment based on their effect IDs.
    ///
    /// This function is a core component of the in-place Heapsort algorithm. It ensures that
    /// the subtree rooted at `root` satisfies the heap property: the effect ID of a parent
    /// node must be greater than or equal to the effect IDs of its children.
    ///
    /// # Arguments
    /// * `root` - The index of the element to sift down.
    /// * `end` - The upper bound (exclusive) of the heap segment in `primary_buffer`.
    ///
    /// # Complexity
    /// - **Time**: O(log n), where n is the distance from `root` to `end`.
    /// - **Space**: O(1), as it operates directly on the existing buffer.
    fn sift_down(&mut self, mut root: usize, end: usize) {
        while root * 2 + 1 < end {
            let mut child = root * 2 + 1; // Left child index

            if child + 1 < end {
                // Access the effect IDs (index 1 of a When node's children)
                let eff_left = self.get(self.primary_buffer[child]).unwrap().children()[1];
                let eff_right = self.get(self.primary_buffer[child + 1]).unwrap().children()[1];

                // If the right child has a larger effect ID, we target it instead
                if eff_left < eff_right {
                    child += 1;
                }
            }

            let eff_root = self.get(self.primary_buffer[root]).unwrap().children()[1];
            let eff_child = self.get(self.primary_buffer[child]).unwrap().children()[1];

            // If the child's effect is larger than the root's, swap and continue down
            if eff_root < eff_child {
                self.primary_buffer.swap(root, child);
                root = child;
            } else {
                break;
            }
        }
    }

    /// Checks for the presence of a complementary pair (P and ¬P) in a sorted list of expressions.
    ///
    /// This helper is used to identify contradictions in `AND` blocks (resulting in `False`)
    /// or tautologies in `OR` blocks (resulting in `True`).
    ///
    /// # Algorithm
    /// The function iterates through the provided IDs. For every `Not` node encountered,
    /// it performs a binary search for its inner atom within the sorted slice.
    ///
    /// # Complexity
    /// - **Time**: O(n log n), where `n` is the number of operands. This is generally
    ///   faster than using a `HashSet` for the small number of operands typically
    ///   found in PDDL expressions.
    /// - **Space**: O(1), as it operates in-place on the stack-allocated slice without
    ///   additional heap allocations.
    ///
    /// # Arguments
    /// * `sorted_ids` - A slice of `ExprId` already sorted by their internal value.
    fn has_complementary_pair(&self, sorted_ids: &[ExprId]) -> bool {
        if sorted_ids.len() < 2 {
            return false;
        }

        for (i, &id) in sorted_ids.iter().enumerate() {
            if let Some(node) = self.get(id) {
                // We look for negation nodes to find their corresponding atoms
                if let ExprEntryKind::Not = node.kind() {
                    let atom_inside_not = node.children()[0];

                    // Since the slice is sorted, binary search allows us to find
                    // the atom in O(log n) time.
                    if sorted_ids.binary_search(&atom_inside_not).is_ok() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Creates a logical `NOT` node (negation).
    ///
    /// This function implements unary simplifications to keep the expression
    /// tree as shallow as possible.
    ///
    /// # Simplifications
    /// - **Constant Folding**: `¬True` becomes `False`, and `¬False` becomes `True`.
    /// - **Double Negation Elimination**: `¬(¬A)` simplifies directly to `A`.
    ///
    /// # Returns
    /// The `ExprId` of the simplified expression.
    pub fn not(&mut self, expr: ExprId) -> ExprId {
        let true_id = self.empty_and();
        let false_id = self.empty_or();

        // 1. Constant Folding
        if expr == true_id {
            return false_id;
        }
        if expr == false_id {
            return true_id;
        }

        // 2. Double Negation Elimination ( !!A -> A )
        // We peek into the store to see if the expression is already a Not node.
        if let Some(node) = self.get(expr) {
            if let ExprEntryKind::Not = node.kind() {
                // By construction, a Not node always has exactly one child.
                return node.children()[0];
            }
        }

        // 3. Internment
        // If no simplification applies, create or retrieve the existing Not node.
        self.intern(ExprEntryKind::Not, &[expr])
    }

    /// Creates an implication node: `(antecedent → consequent)`.
    ///
    /// This function eagerly transforms the implication into its disjunctive
    /// normal form: `(¬A ∨ B)`. By reducing the implication immediately,
    /// the system maximizes Hash-Consing efficiency and leverages existing
    /// optimizations in the `OR` and `NOT` builders.
    ///
    /// # Logical Simplifications
    /// - **Identity**: `(A → A)` simplifies to `True`.
    /// - **False Antecedent**: `(False → B)` is a tautology, simplifies to `True`.
    /// - **True Consequent**: `(A → True)` is a tautology, simplifies to `True`.
    /// - **True Antecedent**: `(True → B)` simplifies to `B`.
    ///
    /// # Returns
    /// An `ExprId` representing the simplified disjunction.
    pub fn imply(&mut self, antecedent: ExprId, consequent: ExprId) -> ExprId {
        let true_id = self.empty_and();
        let false_id = self.empty_or();

        // 1. Trivial Case: Identity (A -> A)
        if antecedent == consequent {
            return true_id;
        }

        // 2. Trivial Case: False Antecedent (False -> B)
        if antecedent == false_id {
            return true_id;
        }

        // 3. Trivial Case: True Consequent (A -> True)
        if consequent == true_id {
            return true_id;
        }

        // 4. Trivial Case: True Antecedent (True -> B)
        if antecedent == true_id {
            return consequent;
        }

        // 5. General Transformation: (A => B)  <=>  (¬A ∨ B)
        let not_a = self.not(antecedent);
        self.or(&[not_a, consequent])
    }

    /// Creates a conditional effect node: `(when condition effect)`.
    ///
    /// This function applies several "smart" simplifications to avoid creating
    /// redundant nodes in the store:
    ///
    /// 1. **Direct Application**: If the condition is always True, it returns the effect.
    /// 2. **No-op (Condition False)**: If the condition is False, the effect never triggers.
    /// 3. **No-op (Empty Effect)**: If the effect is an empty AND (True), it does nothing.
    /// 4. **No-op (False Effect)**: If the effect is an empty OR (False), it results in no change.
    /// 5. **No-op (Identity)**: If the condition and effect are the same, it changes nothing.
    ///
    /// Returns the simplified `ExprId`.
    pub fn when(&mut self, cond: ExprId, eff: ExprId) -> ExprId {
        let true_id = self.empty_and();
        let false_id = self.empty_or();

        // 1. Direct Application: (when True E) -> E
        if cond == true_id {
            return eff;
        }

        // Simplified No-ops (Return empty AND):
        // 2. Condition is False: (when False E)
        // 3. Effect is Empty:    (when C True)
        // 4. Effect is False:    (when C False)
        // 5. Identity:           (when E E)
        if cond == false_id || eff == true_id || eff == false_id || cond == eff {
            return true_id;
        }

        self.intern(ExprEntryKind::When, &[cond, eff])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::VariableId;
    use crate::aiplan4rust::lir::store::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::ExprStore;

    /// Test: (and P True) -> P
    /// Verifies that the neutral element (True) is removed from an AND operation.
    #[test]
    fn test_logical_identity_and_empty() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();

        let root = builder.and(&[p, true_val]);

        assert_eq!(root, p, "AND with True must return the original operand ID");
    }

    /// Test: (or P False) -> P
    /// Verifies that the neutral element (False) is removed from an OR operation.
    #[test]
    fn test_logical_identity_or_empty() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let false_val = builder.empty_or();

        let root = builder.or(&[p, false_val]);

        assert_eq!(root, p, "OR with False must return the original operand ID");
    }

    /// Test: (and P False Q) -> False
    /// Verifies the absorbing element property: any False in an AND chain results in False.
    #[test]
    fn test_and_absorbing_element() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let q = builder.variable(VariableId::from(2));
        let false_val = builder.empty_or();

        let root = builder.and(&[p, false_val, q]);

        assert_eq!(
            root, false_val,
            "AND containing False must be folded to False"
        );
    }

    /// Test: (and a (and b c)) -> (and a b c)
    /// Verifies structural flattening: nested identical logical operations are merged.
    #[test]
    fn test_logical_flattening() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));
        let c = builder.variable(VariableId::from(3));

        let inner = builder.and(&[b, c]);
        let root = builder.and(&[a, inner]);

        let node = builder.get(root).expect("Node should exist");
        assert_eq!(
            node.children().len(),
            3,
            "The AND node should be flattened to 3 children"
        );
    }

    /// Test: (and b a b a) -> (and a b)
    /// Verifies that operands are sorted and deduplicated to ensure Hash-Consing efficiency.
    #[test]
    fn test_logical_deduplication_and_sorting() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));

        let root1 = builder.and(&[b, a, b, a]);
        let root2 = builder.and(&[a, b]);

        assert_eq!(
            root1, root2,
            "Different construction orders must result in the same ExprId"
        );
    }

    /// Test: (and p q (not p)) -> False
    /// Verifies contradiction detection: a set containing both P and NOT P is folded to False.
    #[test]
    fn test_complementary_pair_and() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let q = builder.variable(VariableId::from(2));
        let not_p = builder.not(p);
        let false_val = builder.empty_or();

        let root = builder.and(&[p, q, not_p]);

        assert_eq!(
            root, false_val,
            "P and NOT P in the same AND must result in False"
        );
    }

    /// Test: (and (when c1 eff) (when c2 eff)) -> (when (or c1 c2) eff)
    /// Verifies the fusion of conditional effects sharing the same consequence.
    #[test]
    fn test_finalize_and_with_merge_complex() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let c1 = builder.variable(VariableId::from(1));
        let c2 = builder.variable(VariableId::from(2));
        let eff = builder.variable(VariableId::from(3));

        let w1 = builder.when(c1, eff);
        let w2 = builder.when(c2, eff);
        let root = builder.and(&[w1, w2]);

        let node = builder.get(root).expect("Node should exist");
        assert_eq!(
            node.kind(),
            &ExprEntryKind::When,
            "Multiple Whens with same effect should be merged"
        );

        let condition = node.children()[0];
        let cond_node = builder.get(condition).unwrap();
        assert_eq!(
            cond_node.kind(),
            &ExprEntryKind::Or,
            "Merged condition should be an OR node"
        );
    }

    /// Test: (imply a b) -> (or (not a) b)
    /// Verifies the logical transformation of implications into disjunctions.
    #[test]
    fn test_imply_transformation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let a = builder.variable(VariableId::from(1));
        let b = builder.variable(VariableId::from(2));

        let imp = builder.imply(a, b);
        let not = builder.not(a);
        let manual_or = builder.or(&[not, imp]);

        assert_eq!(
            imp, manual_or,
            "Implication must be equivalent to (!A or B)"
        );
    }

    /// Test: (or P True Q) -> True
    /// Verifies the absorbing element for OR: if any child is True, the result is True.
    #[test]
    fn test_or_absorbing_element() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();

        let root = builder.or(&[p, true_val]);
        assert_eq!(root, true_val);
    }

    /// Test: (not (not P)) -> P
    /// Verifies the double negation elimination rule.
    #[test]
    fn test_double_negation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));

        let not_p = builder.not(p);
        let not_not_p = builder.not(not_p);

        assert_eq!(
            not_not_p, p,
            "Double negation should return the original expression"
        );
    }

    /// Test: (or P (not P)) -> True
    /// Verifies the Law of Excluded Middle: a disjunction of an atom and its negation is True.
    #[test]
    fn test_complementary_pair_or() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let not_p = builder.not(p);
        let true_val = builder.empty_and();

        let root = builder.or(&[p, not_p]);
        assert_eq!(root, true_val);
    }

    /// Test: (and (when c1 e) (when c2 e) P) -> (and (when (or c1 c2) e) P)
    /// Verifies that merging 'When' nodes doesn't lose other siblings in the AND.
    #[test]
    fn test_when_merge_with_siblings() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let c1 = builder.variable(VariableId::from(1));
        let c2 = builder.variable(VariableId::from(2));
        let eff = builder.variable(VariableId::from(3));
        let p = builder.variable(VariableId::from(4));
        let w1 = builder.when(c1, eff);
        let w2 = builder.when(c2, eff);

        let root = builder.and(&[w1, w2, p]);

        let node = builder.get(root).unwrap();
        assert_eq!(
            node.children().len(),
            2,
            "Should contain one 'When' and one 'P'"
        );
    }

    /// Test: (when P P) -> (and)
    /// Verifies that a conditional effect where the condition is the same as the effect
    /// is simplified to a No-op (empty AND), as it doesn't change the state.
    #[test]
    fn test_when_identity_no_op() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));
        let true_val = builder.empty_and();

        // Input: (when P P) | Output: True (empty AND)
        let root = builder.when(p, p);

        assert_eq!(
            root, true_val,
            "A conditional effect (when P P) should be simplified to a No-op"
        );
    }

    /// Test: (and P) -> P
    /// Verifies that an AND/OR with a single child is simplified to the child itself.
    #[test]
    fn test_unary_reduction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));

        assert_eq!(
            builder.and(&[p]),
            p,
            "Unary AND should be reduced to its only child"
        );
        assert_eq!(
            builder.or(&[p]),
            p,
            "Unary OR should be reduced to its only child"
        );
    }

    /// Test: (not False) -> True
    /// Verifies the negation of the False constant.
    #[test]
    fn test_not_false() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let false_val = builder.empty_or();
        let true_val = builder.empty_and();

        assert_eq!(
            builder.not(false_val),
            true_val,
            "NOT False must return True"
        );
    }

    /// Test: (when C False) -> True
    /// Verifies that a conditional effect with a False effect is a No-op.
    #[test]
    fn test_when_false_effect() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let c = builder.variable(VariableId::from(1));
        let false_eff = builder.empty_or(); // Absorption/Identity check
        let true_val = builder.empty_and();

        // Technically, a False effect in PDDL means "impossible" or "no change"
        // depending on context, but here it's treated as a neutral empty_and.
        assert_eq!(builder.when(c, false_eff), true_val);
    }

    /// Test: (not (not (not P))) -> (not P)
    /// Verifies that triple negation reduces to a single negation.
    #[test]
    fn test_triple_negation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let p = builder.variable(VariableId::from(1));

        let not_p = builder.not(p);
        let not_not_p = builder.not(not_p);
        let not_not_not_p = builder.not(not_not_p);

        assert_eq!(
            not_not_not_p, not_p,
            "Triple negation should reduce to a single NOT node"
        );
    }
}
