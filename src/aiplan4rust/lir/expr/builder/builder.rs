//! Expression Builder Module
//!
//! This module provides the infrastructure for programmatically constructing
//! expression trees within an [`ExprStore`].
//!
//! The primary tool is the [`ExprBuilder`] struct, which serves as a specialized
//! factory to ensure that expressions are built efficiently, minimizing
//! heap allocations by reusing internal buffers.
//!
//! # Key Features
//!
//! - **Buffer Reuse**: Uses pre-allocated vectors to handle child nodes for
//!   n-ary operations (Sum, Product, And, Or).
//! - **Precision Handling**: Includes a standard [`EPSILON`] for robust
//!   floating-point comparisons in planning domains.
//! - **Variable Scoping**: Integrated support for [`TypedList`] to manage
//!   parameters and quantified variables.
//!
//! # Error Handling
//!
//! Operations within this module return results that may fail with:
//! - [`ExprBuilderError`]: For logical construction errors (e.g., arity mismatch).
//! - [`StorerError`]: For low-level storage issues within the interned memory.

use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::expr::{ExprId, ExprKind, ExprNode, ExprStore};

/// The debug tolerance used for floating-point comparisons within the builder.
///
/// This value is used by comparison utilities (e.g., `is_eq`, `is_gt`) to
/// mitigate precision issues inherent in floating-point representations
/// during PDDL numeric operations.
pub const EPSILON: f64 = 1e-9;

/// A stateful builder for constructing expressions within an [`ExprStore`].
///
/// `ExprBuilder` provides a high-level API to create and manipulate
/// Linear Intermediate Representation (LIR) expressions. It manages internal
/// buffers to minimize memory allocations when building complex or
/// nested expression trees.
pub struct ExprBuilder<'a> {
    /// Reference to the storage where the expressions are interned.
    pub(crate) store: &'a mut ExprStore,
    /// Reusable storage for temporary [`ExprId`]s, typically used for children of n-ary nodes.
    pub(crate) primary_buffer: Vec<ExprId>,
    /// A secondary scratchpad for complex operations requiring two sets of expressions.
    pub(crate) secondary_buffer: Vec<ExprId>,
    /// Manages scoped variables and their types for quantified expressions.
    pub(crate) vars_buffer: TypedList<VariableId, TypeId>,
}
impl<'a> ExprBuilder<'a> {
    /// Creates a new builder instance bound to a mutable reference of an [`ExprStore`].
    ///
    /// # Arguments
    /// * `old` - A mutable reference to the expression storage where nodes are interned.
    ///
    /// # Returns
    /// A new `ExprBuilder` with pre-allocated internal buffers to minimize heap pressure
    /// during expression construction.
    pub fn new(store: &'a mut ExprStore) -> Self {
        Self {
            store,
            primary_buffer: Vec::with_capacity(32),
            secondary_buffer: Vec::with_capacity(32),
            vars_buffer: TypedList::with_capacity(16),
        }
    }

    /// Provides mutable access to the underlying storage.
    ///
    /// This is an internal helper used when direct old manipulation is required
    /// outside of standard interning.
    pub(crate) fn store(&mut self) -> &mut ExprStore {
        self.store
    }

    /// Interns a new expression node into the storage.
    ///
    /// This is a shortcut for the old's interning mechanism. It performs
    /// hash-consing to ensure that identical expressions share the same ID.
    ///
    /// # Arguments
    /// * `kind` - The variant defining the type of expression.
    /// * `children` - A slice of identifiers representing the child nodes.
    ///
    /// # Returns
    /// The unique [`ExprId`] associated with the interned node.
    #[inline]
    pub fn intern(&mut self, kind: ExprKind, children: &[ExprId]) -> ExprId {
        self.store().intern(kind, children)
    }

    /// Retrieves an ergonomic view of an expression node.
    ///
    /// This method returns an `Option`, making it suitable for routine checks
    /// where an ID might not exist in the old.
    ///
    /// # Arguments
    /// * `id` - The identifier of the expression to retrieve.
    ///
    /// # Returns
    /// An `Option` containing an [`ExprNode`] if the ID is valid.
    #[inline]
    pub fn get(&self, id: ExprId) -> Option<ExprNode<'_>> {
        self.store.get(id)
    }

    /// Fetches an expression entry or returns a storage-level error.
    ///
    /// Unlike [`get`], this method is designed for operations where the ID
    /// is strictly expected to exist. It propagates the old's error type.
    ///
    /// # Arguments
    /// * `id` - The identifier of the expression to fetch.
    ///
    /// # Returns
    /// * `Ok(ExprNodeRef)` - The view on the requested node.
    /// * `Err(StorerError)` - If the ID is invalid or the old is corrupted.
    #[inline]
    pub fn fetch(&self, id: ExprId) -> Result<ExprNode<'_>, StorerError> {
        self.store.fetch(id)
    }

    /// Reconstructs an expression using its kind and existing child identifiers.
    ///
    /// This method acts as a high-level dispatcher that routes the reconstruction through
    /// the builder's "Smart Constructors" (e.g., `and`, `at_start`, `forall`). This ensures
    /// that any semantic validation, simplification, or normalization logic is reapplied
    /// during reconstruction.
    ///
    /// # Arguments
    ///
    /// * `kind` - The [`ExprKind`] of the node to reconstruct.
    /// * `children` - A slice of [`ExprId`] representing the already interned children
    ///   of this expression.
    ///
    /// # Returns
    ///
    /// * `Ok(ExprId)` - The identifier of the reconstructed (and potentially simplified) node.
    /// * `Err(ExprBuilderError)` - If the reconstruction violates semantic rules (e.g.,
    ///   illegal temporal nesting or duplicate variable declarations in quantifiers).
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - A temporal operator (`AtStart`, `AtEnd`, `Overall`) is nested illegally.
    /// - A quantifier (`Forall`, `Exists`) introduces invalid variable scopes.
    pub fn reconstruct(
        &mut self,
        kind: ExprKind,
        children: &[ExprId],
    ) -> Result<ExprId, ExprBuilderError> {
        let id = match kind {
            // --- 1. Variadic Logical and Arithmetic Operators ---
            // Uses smart constructors to trigger potential simplifications (e.g., flattening)
            ExprKind::And => self.and(children),
            ExprKind::Or => self.or(children),
            ExprKind::Arithmetic(op) => self.arithmetic(op, children),

            // --- 2. Binary Operators (2 children) ---
            ExprKind::Imply => self.imply(children[0], children[1]),
            ExprKind::When => self.when(children[0], children[1]),
            ExprKind::Comparison(op) => self.comparison(op, children[0], children[1]),
            ExprKind::Assignment(op) => self.assignment(op, children[0], children[1]),

            ExprKind::TaskOrderingConstraint(op) => {
                self.intern(ExprKind::TaskOrderingConstraint(op), children)
            }

            // --- 3. Unary Operators (1 child) ---
            ExprKind::Not => self.not(children[0]),
            // Fallible temporal operators: propagation of Result is required
            ExprKind::AtStart => self.at_start(children[0])?,
            ExprKind::AtEnd => self.at_end(children[0])?,
            ExprKind::Overall => self.overall(children[0])?,

            // Metadata and Metric nodes (Direct interning)
            ExprKind::Preference | ExprKind::IsViolated | ExprKind::Metric(_) => {
                self.intern(kind, children)
            }

            // --- 4. Quantifiers (Variables + 1 child) ---
            ExprKind::Forall(vars) => self.forall(vars, children[0])?,
            ExprKind::Exists(vars) => self.exists(vars, children[0])?,

            // --- 5. Nodes with Skeletons (AtomicFormula, Function, Task) ---
            // Note: children[0] is typically the symbol, children[1..] are the arguments.
            ExprKind::AtomicFormula(skel) => self.intern(ExprKind::AtomicFormula(skel), children),
            ExprKind::Function(skel) => self.intern(ExprKind::Function(skel), children),
            ExprKind::Task(skel) => self.intern(ExprKind::Task(skel), children),

            // --- 6. Complex Temporal and HTN Nodes ---
            ExprKind::Always
            | ExprKind::Sometime
            | ExprKind::Within
            | ExprKind::AtMostOnce
            | ExprKind::SometimeAfter
            | ExprKind::SometimeBefore
            | ExprKind::AlwaysWithin
            | ExprKind::HoldDuring
            | ExprKind::HoldAfter
            | ExprKind::TimedInitialLiteral
            | ExprKind::LabeledTask
            | ExprKind::Serial
            | ExprKind::Parallel
            | ExprKind::Length => self.intern(kind, children),

            // --- 7. Leaf Nodes (Terminal nodes) ---
            // Objects, Numbers, Variables, etc. take no children in the buffer.
            leaf_kind => self.intern(leaf_kind, &[]),
        };
        Ok(id)
    }

    /// Returns true if two values are nearly equal within [`Self::EPSILON`].
    ///
    /// # Arguments
    /// * `val` - The primary value to compare.
    /// * `other` - The secondary value to compare against.
    ///
    /// # Returns
    /// `true` if the absolute difference between `val` and `other` is less than or
    /// equal to the defined epsilon.
    #[inline]
    pub fn is_eq(&self, val: f64, other: f64) -> bool {
        (val - other).abs() <= EPSILON
    }

    /// Returns true if `val` is significantly greater than `other` (beyond [`Self::EPSILON`]).
    ///
    /// # Arguments
    /// * `val` - The value to check.
    /// * `other` - The threshold value.
    ///
    /// # Returns
    /// `true` if `val` exceeds `other` by more than epsilon.
    #[inline]
    pub fn is_gt(&self, val: f64, other: f64) -> bool {
        val > other + EPSILON
    }

    /// Returns true if `val` is significantly less than `other` (beyond [`Self::EPSILON`]).
    ///
    /// # Arguments
    /// * `val` - The value to check.
    /// * `other` - The threshold value.
    ///
    /// # Returns
    /// `true` if `val` is lower than `other` by more than epsilon.
    #[inline]
    pub fn is_lt(&self, val: f64, other: f64) -> bool {
        val < other - EPSILON
    }

    /// Returns true if `val` is greater than or nearly equal to `other`.
    ///
    /// # Arguments
    /// * `val` - The value to check.
    /// * `other` - The threshold value.
    ///
    /// # Returns
    /// `true` if `val` is greater than `other` or within epsilon of it.
    #[inline]
    pub fn is_ge(&self, val: f64, other: f64) -> bool {
        val >= other - EPSILON
    }

    /// Returns true if `val` is less than or nearly equal to `other`.
    ///
    /// # Arguments
    /// * `val` - The value to check.
    /// * `other` - The threshold value.
    ///
    /// # Returns
    /// `true` if `val` is less than `other` or within epsilon of it.
    #[inline]
    pub fn is_le(&self, val: f64, other: f64) -> bool {
        val <= other + EPSILON
    }

    /// Special check for zero-equivalence within [`Self::EPSILON`].
    ///
    /// # Arguments
    /// * `val` - The value to check for zero-equivalence.
    ///
    /// # Returns
    /// `true` if the absolute value of `val` is within epsilon of 0.0.
    #[inline]
    pub fn is_zero(&self, val: f64) -> bool {
        val.abs() <= EPSILON
    }

    /// Checks if a value is significantly negative (less than -[`Self::EPSILON`]).
    ///
    /// # Arguments
    /// * `val` - The value to check.
    ///
    /// # Returns
    /// `true` if `val` is less than the negative epsilon. Values between
    /// -epsilon and 0.0 are considered non-negative (zero).
    #[inline]
    pub fn is_neg(&self, val: f64) -> bool {
        val < -EPSILON
    }

    /// Sorts a given buffer by `ExprId` to enable deduplication and canonicalization.
    ///
    /// This is a specialized implementation of the Heapsort algorithm. Sorting operands
    /// by their unique internal identifiers ensures that expressions are stored
    /// in a consistent (canonical) order.
    ///
    /// # Arguments
    /// * `buffer` - A mutable reference to the buffer to sort (primary or secondary).
    /// * `len` - The number of elements in the buffer to sort.
    pub(crate) fn sort_buffer_by_id(buffer: &mut Vec<ExprId>, len: usize) {
        if len <= 1 {
            return;
        }

        // Phase 1: Build a max-heap
        for start in (0..len / 2).rev() {
            Self::sift_down_by_id(buffer, start, len);
        }

        // Phase 2: Extract elements
        for end in (1..len).rev() {
            buffer.swap(0, end);
            Self::sift_down_by_id(buffer, 0, end);
        }
    }

    /// Restores the max-heap property for a sub-section of the provided buffer.
    ///
    /// This is a core utility for heap-based operations (like heapsort or priority queue management).
    /// It moves the element at the `root` index down the tree until it is no longer smaller
    /// than its children, ensuring the max-heap invariant is maintained.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable reference to the `Vec<ExprId>` representing the heap-ordered tree.
    /// * `root` - The index of the element to be sifted down.
    /// * `end` - The upper bound (exclusive) of the heap within the buffer.
    ///
    /// # Implementation Details
    ///
    /// The function uses a standard binary heap representation where:
    /// - Left child: `root * 2 + 1`
    /// - Right child: `root * 2 + 2`
    ///
    /// At each step, it identifies the largest child and swaps it with the parent if the
    /// parent is smaller, continuing the process iteratively until the heap property is restored.
    ///
    /// # Complexity
    ///
    /// * **Time complexity**: $O(\log n)$, where $n$ is the distance between `root` and `end`.
    /// * **Space complexity**: $O(1)$ as it operates in-place.
    fn sift_down_by_id(buffer: &mut Vec<ExprId>, mut root: usize, end: usize) {
        while root * 2 + 1 < end {
            let mut child = root * 2 + 1; // Left child

            // If right child exists and is greater than left child, pick right child
            if child + 1 < end && buffer[child] < buffer[child + 1] {
                child += 1;
            }

            // If the root is smaller than the largest child, swap and continue
            if buffer[root] < buffer[child] {
                buffer.swap(root, child);
                root = child;
            } else {
                // Max-heap property is satisfied
                break;
            }
        }
    }

    /// Extracts a literal floating-point value from an expression ID if it points to a number.
    ///
    /// This is a convenience helper that traverses the `ExprStore` to check if a specific
    /// [`ExprId`] corresponds to a [`ExprKind::Number`].
    ///
    /// # Arguments
    ///
    /// * `id` - The [`ExprId`] of the expression to inspect.
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The inner value if the expression is a numeric literal.
    /// * `None` - If the expression does not exist or is not a number (e.g., it's a variable or another operation).
    pub(crate) fn get_number(&self, id: ExprId) -> Option<f64> {
        self.get(id).and_then(|n| {
            if let ExprKind::Number(v) = n.kind() {
                Some(v.into_inner())
            } else {
                None
            }
        })
    }
}
