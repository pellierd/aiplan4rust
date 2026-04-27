//! # Arithmetic Module: Construction and Optimization Pipeline
//!
//! This module provides a high-performance API for creating arithmetic expressions
//! within the LIR (Linear Intermediate Representation) store. It transforms raw
//! operations into simplified, canonical forms.
//!
//! ## Design Philosophy
//!
//! ### 1. Zero-Allocation (Heap Stability)
//! The builder is engineered to avoid heap allocations during the construction phase.
//! It utilizes two persistent internal buffers (`primary_buffer` and `secondary_buffer`)
//! which are reused across every `arithmetic` call. This eliminates "heap churn"
//! and ensures predictable performance under heavy workloads.
//!
//! ### 2. Hash-Consing & Uniqueness
//! Every generated expression is unique within the store. If two different code paths
//! create the same semantic expression (e.g., `(+ 1 2 x)` and `(+ x 3)`), they will
//! receive the exact same [`ExprId`]. This drastically reduces the memory footprint
//! and accelerates subsequent comparisons (O(1) pointer equality).
//!
//! ### 3. Four-Phase Transformation Pipeline
//!
//! The construction engine follows a rigorous pipeline to ensure optimality:
//!
//! | Phase | Name | Role |
//! | :--- | :--- | :--- |
//! | **1** | **Early Fold** | Intercepts errors (`NaN`, `/0`) and symbolic identities (`x-x`, `x*0`) without using buffers. |
//! | **2** | **Flattening** | Flattens associative operations. `(a + (b + c))` becomes `(+ a b c)`. Also handles PDDL unary negation. |
//! | **3** | **Constant Folding** | In-place numerical reduction. Merges all constant literals into a single value. |
//! | **4** | **Finalization** | Sorts operands (commutative canonization), reduces unary forms, and performs final interning. |
//!
//! ## Transformation Example
//!
//! ```ignore
//! // Input: arithmetic(Add, &[x, arithmetic(Add, &[2, 3])])
//! // 1. Phase 2 (Flattening) : [x, 2, 3]
//! // 2. Phase 3 (Folding)    : [x, 5.0]
//! // 3. Phase 4 (Sorting)    : [5.0, x] (based on internal IDs)
//! // Result: ExprId pointing to (+ 5.0 x)
//! ```

use crate::aiplan4rust::lang::ArithmeticOp;
use crate::aiplan4rust::lir::store::expr::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use ordered_float::OrderedFloat;

impl<'a> ExprBuilder<'a> {
    /// Constructs an optimized arithmetic expression: `(op operands...)`.
    ///
    /// This function orchestrates a multi-step pipeline to ensure the resulting
    /// [`ExprId`] points to the most simplified and canonical form of the expression.
    /// By utilizing *Hash-Consing*, semantically equivalent expressions (e.g., `x + y`
    /// and `y + x`) will always share the same unique [`ExprId`].
    ///
    /// # Pipeline Phases
    ///
    /// 1. **Early Fold (Fast Path)**: Immediate resolution without buffer usage.
    ///    It intercepts error cases (`NaN`, division by zero) and complex symbolic
    ///    identities (e.g., `x - x` results in `0.0`, `x + 0` results in `x`).
    ///
    /// 2. **Collection & Flattening**: Structural normalization. Associative operations
    ///    (`Add`, `Mul`) are flattened into the `primary_buffer`.
    ///    *Example: `(+ (+ a b) c)` becomes `(+ a b c)`.*
    ///
    /// 3. **In-place Constant Folding**: Aggressive numerical reduction. Iterates through
    ///    the buffer to aggregate literals and handle absorbing elements (e.g., `0 * x`
    ///    results in `0`). It strictly maintains operand order for non-commutative
    ///    operations (`Sub`, `Div`).
    ///
    /// 4. **Finalization & Interning**:
    ///    - Handles unary reduction (e.g., `(+ x)` simplifies to `x`).
    ///    - Applies a sorting algorithm (Heapsort) on the buffer for commutative operations.
    ///    - Deduplicates the final expression in the store via `intern`.
    ///
    /// # Performance
    ///
    /// The pipeline is designed to be **Zero-Alloc**. It reuses the `ExprBuilder` internal
    /// buffers to avoid dynamic `Vec` allocations on the heap during transformation phases.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] to apply (Add, Sub, Mul, or Div).
    /// * `operands` - A slice of [`ExprId`] representing the terms of the expression.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier of the simplified expression in the store.
    pub fn arithmetic(&mut self, op: ArithmeticOp, operands: &[ExprId]) -> ExprId {
        // --- PHASE 1: FAST PATH (Identities & Errors) ---
        if let Some(id) = self.early_fold_nan(operands) {
            return id;
        }
        if let Some(id) = self.early_fold_div_by_zero(op, operands) {
            return id;
        }
        if let Some(id) = self.early_fold_identities(op, operands) {
            return id;
        }

        // --- PHASE 2: COLLECTION & FLATTENING ---
        self.flatten_operands(op, operands);

        // --- PHASE 3: NUMERICAL REDUCTION ---
        if let Some(id) = self.fold_constants(op) {
            return id;
        }

        // --- PHASE 4: FINALIZATION (Sorting & Interning) ---
        self.finalize(op)
    }

    /// Checks if any operand is a `NaN` (Not-a-Number) and returns its ID.
    ///
    /// In PDDL arithmetic, `NaN` values are considered "poisonous" and propagate
    /// upwards. If a `NaN` is detected early, the entire expression can be
    /// folded into that `NaN` constant without further processing.
    ///
    /// # Arguments
    ///
    /// * `operands` - A slice of [`ExprId`] representing the children of the arithmetic expression.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - The ID of the first `NaN` constant found among the operands.
    /// * `None` - If no `NaN` constant is detected, allowing the pipeline to continue.
    fn early_fold_nan(&self, operands: &[ExprId]) -> Option<ExprId> {
        for &id in operands {
            if let Some(node) = self.get(id) {
                if let ExprEntryKind::Number(n) = node.kind() {
                    if n.into_inner().is_nan() {
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    /// Handles division by zero during the early construction phase.
    ///
    /// If the operation is a division and the divisor is a numeric literal that
    /// evaluates to nearly `0.0` (within epsilon), it immediately returns a `NaN`
    /// constant.
    ///
    /// This prevents invalid runtime calculations and protects the solver against
    /// numerical explosions (overflows) caused by dividing by extremely small
    /// values that are physically insignificant in the model.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] being performed.
    /// * `operands` - A slice of [`ExprId`] containing the dividend and the divisor.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - An ID pointing to a canonical `NaN` constant if a
    ///   near-zero divisor is detected.
    /// * `None` - If the divisor is not zero or not a literal, allowing the pipeline to proceed.
    fn early_fold_div_by_zero(&mut self, op: ArithmeticOp, operands: &[ExprId]) -> Option<ExprId> {
        if op == ArithmeticOp::Div && operands.len() == 2 {
            // We only check the second operand (the divisor).
            let divisor_id = operands[1];
            if let Some(val) = self.get_number(divisor_id) {
                // Use epsilon-aware check to catch -0.0, 0.0, and infinitesimal residues.
                if self.is_zero(val) {
                    return Some(self.number(f64::NAN));
                }
            }
        }
        None
    }

    /// Simplifies symbolic and algebraic identities where the result is independent of variable values.
    ///
    /// This method acts as an epsilon-aware "fast path" to reduce expressions based on
    /// mathematical properties before they enter the more expensive flattening and
    /// interning phases.
    ///
    /// ### 1. Symbolic Identities (Self-Identity)
    /// Leverages **Hash-Consing**: if two operands share the same [`ExprId`], they are
    /// structurally identical.
    /// - `x - x => 0.0`
    /// - `x / x => 1.0` (Note: Division by zero is pre-handled)
    ///
    /// ### 2. Algebraic Identities (Epsilon-aware)
    /// Uses robust comparison helpers (`is_zero`, `is_eq`) to handle floating-point residues:
    /// - **Neutral Elements**: `x + ~0 => x`, `x * ~1 => x`, `x - ~0 => x`, `x / ~1 => x`.
    /// - **Absorbing Elements**: `x * ~0 => 0.0`, `~0 / x => 0.0`.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] to evaluate.
    /// * `operands` - A slice of [`ExprId`] representing the operation's children.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - The ID of the simplified expression.
    /// * `None` - If no identities are applicable, allowing the pipeline to continue.
    fn early_fold_identities(&mut self, op: ArithmeticOp, operands: &[ExprId]) -> Option<ExprId> {
        // --- CASE 1: SYMBOLIC IDENTITIES (x op x) ---
        // Pointer equality on IDs guarantees structural identity.
        if operands.len() == 2 && operands[0] == operands[1] {
            match op {
                ArithmeticOp::Sub => return Some(self.number(0.0)),
                ArithmeticOp::Div => return Some(self.number(1.0)),
                _ => {}
            }
        }

        // --- CASE 2: NEUTRAL AND ABSORBING ELEMENTS ---
        // Targets binary operations involving at least one epsilon-equivalent literal.
        if operands.len() == 2 {
            let left = operands[0];
            let right = operands[1];
            let val_left = self.get_number(left);
            let val_right = self.get_number(right);

            match op {
                ArithmeticOp::Add => {
                    if let Some(v) = val_left {
                        if self.is_zero(v) {
                            return Some(right);
                        }
                    }
                    if let Some(v) = val_right {
                        if self.is_zero(v) {
                            return Some(left);
                        }
                    }
                }
                ArithmeticOp::Sub => {
                    if let Some(v) = val_right {
                        if self.is_zero(v) {
                            return Some(left);
                        }
                    }
                }
                ArithmeticOp::Mul => {
                    if let Some(v) = val_left {
                        if self.is_eq(v, 1.0) {
                            return Some(right);
                        }
                        if self.is_zero(v) {
                            return Some(self.number(0.0));
                        }
                    }
                    if let Some(v) = val_right {
                        if self.is_eq(v, 1.0) {
                            return Some(left);
                        }
                        if self.is_zero(v) {
                            return Some(self.number(0.0));
                        }
                    }
                }
                ArithmeticOp::Div => {
                    if let Some(v) = val_right {
                        if self.is_eq(v, 1.0) {
                            return Some(left);
                        }
                    }
                    if let Some(v) = val_left {
                        if self.is_zero(v) {
                            return Some(self.number(0.0));
                        }
                    }
                }
            }
        }
        None
    }

    /// Flattens nested associative operations and collects operands into the primary buffer.
    ///
    /// This method implements structural normalization for associative operators (specifically
    /// `Add` and `Mul`). If an operand's operator matches the parent's operator, its children
    /// are hoisted directly into the parent, transforming nested structures like `(a + (b + c))`
    /// into a flat form `(+ a b c)`.
    ///
    /// # Normalization Logic
    ///
    /// - **Unary Subtraction**: If a `Sub` operation is provided with a single operand `(- x)`,
    ///   it is preemptively transformed into a binary subtraction `(0.0 - x)`. This ensures
    ///   that negation is correctly handled by the constant folder and the interner.
    /// - **Associative Operations**: For `Add` and `Mul`, the method performs a single-level
    ///   flattening by inspecting child nodes.
    /// - **Non-Associative Operations**: For `Sub` (binary) and `Div`, operands are copied
    ///   as-is to preserve mathematical order.
    ///
    /// # Memory Management
    ///
    /// To maintain a **Zero-Alloc** profile, this method utilizes the builder's internal
    /// `primary_buffer` and `secondary_buffer`. This prevents dynamic heap allocations
    /// during tree traversal.
    ///
    /// # Arguments
    ///
    /// * `op` - The current [`ArithmeticOp`] being processed.
    /// * `operands` - The initial slice of [`ExprId`] to be flattened and collected.
    fn flatten_operands(&mut self, op: ArithmeticOp, operands: &[ExprId]) {
        self.primary_buffer.clear();

        // Preemptive normalization of Unary Subtraction
        // Converts (- x) into (0.0 - x) to ensure consistent folding and evaluation.
        if op == ArithmeticOp::Sub && operands.len() == 1 {
            let zero = self.number(0.0);
            self.primary_buffer.push(zero);
            self.primary_buffer.push(operands[0]);
            return;
        }

        // If the operation is not associative (e.g., Sub, Div),
        // flattening is mathematically invalid; perform a simple copy.
        if !op.is_associative() {
            self.primary_buffer.extend_from_slice(operands);
            return;
        }

        for &id in operands {
            if let Some(node) = self.store.get(id) {
                if let ExprEntryKind::Arithmetic(child_op) = node.kind() {
                    if *child_op == op {
                        self.secondary_buffer.clear();
                        self.secondary_buffer.extend_from_slice(node.children());
                        self.primary_buffer
                            .extend_from_slice(&self.secondary_buffer);
                        continue;
                    }
                }
            }
            self.primary_buffer.push(id);
        }
    }

    /// Numerically reduces constant literals within the primary buffer using an in-place linear pass.
    ///
    /// This method aggregates literal numbers into a single constant where mathematically possible,
    /// using epsilon-aware logic to handle floating-point imprecision. It employs a "write pointer"
    /// strategy to update the `primary_buffer` in-place, avoiding heap allocations.
    ///
    /// # Optimization Logic
    ///
    /// 1. **Short-Circuiting**: Immediately returns if a "poisonous" or "absorbing" value is
    ///    encountered (e.g., `NaN` propagates, or `~0.0` in `Mul` absorbs the expression).
    /// 2. **Commutative Folding**: For `Add` and `Mul`, all constants are aggregated
    ///    regardless of position.
    /// 3. **Positional Folding**: For `Sub` and `Div`, folding only occurs at the head
    ///    before any variables are encountered, ensuring `(10 - x) - 2` isn't incorrectly
    ///    folded without reassociation.
    /// 4. **Epsilon Robustness**: Uses `is_zero` and `is_eq` to prevent micro-residues
    ///    (e.g., `1e-17`) from surviving as independent nodes, keeping the expression tree clean.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] determining the folding rules.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - If the expression simplifies to a single constant or error (NaN).
    /// * `None` - If folding completes, leaving remaining operands in the `primary_buffer`.
    fn fold_constants(&mut self, op: ArithmeticOp) -> Option<ExprId> {
        let mut constant_part: Option<f64> = None;
        let mut has_variable = false;
        let mut write_idx = 0;

        let len = self.primary_buffer.len();

        for i in 0..len {
            let id = self.primary_buffer[i];
            match self.get_number(id) {
                Some(v) => {
                    // Phase 3.1: NaN Propagation
                    if v.is_nan() {
                        return Some(id);
                    }

                    // Phase 3.2: Absorption (Multiplication by ~0.0)
                    if op == ArithmeticOp::Mul && self.is_zero(v) {
                        return Some(self.number(0.0));
                    }

                    match (op, constant_part) {
                        // Commutative aggregation (Add, Mul)
                        (ArithmeticOp::Add, c) => constant_part = Some(c.unwrap_or(0.0) + v),
                        (ArithmeticOp::Mul, c) => constant_part = Some(c.unwrap_or(1.0) * v),

                        // Non-commutative positional folding (Sub, Div)
                        (ArithmeticOp::Sub | ArithmeticOp::Div, None) if i == 0 => {
                            constant_part = Some(v);
                        }
                        (ArithmeticOp::Sub, Some(c)) if !has_variable => {
                            constant_part = Some(c - v)
                        }
                        (ArithmeticOp::Div, Some(c)) if !has_variable => {
                            // Robust division by zero check
                            if self.is_zero(v) {
                                return Some(self.number(f64::NAN));
                            }
                            constant_part = Some(c / v);
                        }

                        // Identity values (nearly 0 or 1) that do not advance the write pointer
                        (ArithmeticOp::Sub, _) if self.is_zero(v) => {}
                        (ArithmeticOp::Div, _) if self.is_eq(v, 1.0) => {}

                        // Literal cannot be folded at this position
                        _ => {
                            self.primary_buffer[write_idx] = id;
                            write_idx += 1;
                        }
                    }
                }
                None => {
                    // Irreducible operand (Variable or sub-expression)
                    self.primary_buffer[write_idx] = id;
                    write_idx += 1;
                    has_variable = true;

                    // Special case: ~0 / x => 0.0
                    if op == ArithmeticOp::Div && constant_part.map_or(false, |c| self.is_zero(c)) {
                        return Some(self.number(0.0));
                    }
                }
            }
        }

        // Truncate and re-insert the aggregated constant
        self.primary_buffer.truncate(write_idx);
        self.reinsert_constant_in_buffer(op, constant_part);
        None
    }

    /// Re-inserts the accumulated numeric constant into the primary buffer at the mathematically correct position.
    ///
    /// This is the final step of the constant folding phase. It determines whether the
    /// accumulated constant is significant enough (outside epsilon) to be included in the
    /// final expression or if it can be safely omitted as a neutral element.
    ///
    /// # Positioning Logic
    ///
    /// The placement of the constant is vital for maintaining semantic correctness:
    /// - **Commutative (`Add`, `Mul`)**: The constant is appended. A subsequent sorting
    ///   phase ensures a canonical order for Hash-Consing.
    /// - **Positional (`Sub`, `Div`)**: The accumulated constant represents the base value
    ///   (minuend or dividend) and is inserted at **index 0**.
    ///
    /// # Identity Elimination
    ///
    /// To keep the expression tree lean, neutral elements (e.g., `+ ~0.0`, `* ~1.0`) are
    /// discarded unless the buffer is otherwise empty. In the case of an empty buffer,
    /// the neutral element is kept to represent the operation's identity (e.g., `(+) -> 0.0`).
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] determining the positioning and neutral element rules.
    /// * `constant_part` - The optional accumulated `f64` result from the folding pass.
    fn reinsert_constant_in_buffer(&mut self, op: ArithmeticOp, constant_part: Option<f64>) {
        if let Some(c) = constant_part {
            // Identify if the constant is an epsilon-aware neutral element (identity)
            let is_neutral = match op {
                ArithmeticOp::Add => self.is_zero(c),
                ArithmeticOp::Mul => self.is_eq(c, 1.0),
                _ => false,
            };

            // Re-insert if it's significant, or if it's the only remaining value.
            // Keeping it when the buffer is empty ensures (+ ) -> 0.0 or (* ) -> 1.0.
            if !is_neutral || self.primary_buffer.is_empty() {
                let const_id = self.number(c);

                if op.is_commutative() {
                    // Commutative: append to the end (canonical sorting follows in finalize)
                    self.primary_buffer.push(const_id);
                } else {
                    // Positional: insert at the head to act as the base term (minuend/dividend)
                    self.primary_buffer.insert(0, const_id);
                }
            }
        }
    }

    /// Finalizes the arithmetic expression by performing canonicalization and interning the result.
    ///
    /// This is the final stage of the arithmetic pipeline. It ensures that the expression
    /// is in its simplest form and that its representation is unique within the [`ExprStore`].
    ///
    /// # Finalization Steps
    ///
    /// 1. **Empty Case (Identity Resolution)**: If no operands remain after folding (e.g.,
    ///    neutral elements were eliminated), it returns the mathematical identity for the
    ///    operator (`0.0` for addition/subtraction, `1.0` for multiplication/division).
    ///
    /// 2. **Unary Reduction**: Simplifies expressions with a single operand. For example,
    ///    `(+ x)` is reduced directly to `x`. Note that for non-commutative operations
    ///    like `Sub`, a single operand `(- x)` is preserved as a negation.
    ///
    /// 3. **Canonicalization (Sorting)**: For commutative operations (`Add`, `Mul`),
    ///    operands are sorted by their [`ExprId`]. This ensures that `(x + y)` and `(y + x)`
    ///    result in the same structural representation, enabling perfect deduplication.
    ///
    /// 4. **Interning & Buffer Recovery**: The finalized operand list is interned into the
    ///    store. To maintain efficiency, the `primary_buffer` is temporarily moved to
    ///    avoid cloning, then cleared and returned to the builder to be reused for
    ///    future operations.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] characterizing the expression.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for the interned expression.
    fn finalize(&mut self, op: ArithmeticOp) -> ExprId {
        let len = self.primary_buffer.len();

        // 1. Handle empty operand lists by returning the operator's default identity value.
        if len == 0 {
            let val = match op {
                ArithmeticOp::Add | ArithmeticOp::Sub => 0.0,
                ArithmeticOp::Mul | ArithmeticOp::Div => 1.0,
            };
            return self.number(val);
        }

        // 2. Unary reduction: If only one operand exists, return it directly (e.g., +x -> x).
        // Note: Non-commutative operators like Sub/Div usually require special
        // handling or are preserved as unary operations.
        if len == 1 {
            return self.primary_buffer[0];
        }

        // 3. Canonicalize commutative operations to ensure structural uniqueness.
        if op.is_commutative() {
            Self::sort_buffer_by_id(&mut self.primary_buffer, len);
        }

        // 4. Intern the final expression.
        // We use `std::mem::take` to move the buffer content without allocation.
        let mut data = std::mem::take(&mut self.primary_buffer);
        let id = self.intern(ExprEntryKind::Arithmetic(op), &data);

        // Clear and restore the buffer to the builder for reuse (zero-alloc strategy).
        data.clear();
        self.primary_buffer = data;

        id
    }

    /// Creates a numeric literal node with mandatory NaN and Epsilon-aware Zero normalization.
    ///
    /// This function is generic over `T: Into<f64>`, allowing transparent use of
    /// native `f64`, `f32`, or `OrderedFloat<f64>`.
    ///
    /// To ensure perfect deduplication (hash-consing) and numerical stability, this
    /// function normalizes floating-point values into canonical representations:
    ///
    /// 1. **NaN Normalization**: All variations of `NaN` are collapsed into a single representation.
    /// 2. **Epsilon-aware Zero Normalization**: Any value within the builder's epsilon
    ///    range (e.g., -1e-9 to 1e-9) is snapped to exactly `0.0`. This includes `-0.0`.
    ///
    /// This normalization is critical. Without it, infinitesimal residues (floating-point noise)
    /// would create unique [`ExprId`]s, polluting the store and breaking structural
    /// equality checks between semantically identical expressions.
    ///
    /// # Arguments
    ///
    /// * `value` - The floating-point value to wrap and normalize. Accepts any type
    ///   implementing `Into<f64>`.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for this normalized numeric constant.
    pub fn number<T>(&mut self, value: T) -> ExprId
    where
        T: Into<f64>,
    {
        let val_f64 = value.into();

        let normalized = if val_f64.is_nan() {
            // Normalize to a single canonical NaN representation
            OrderedFloat(f64::NAN)
        } else if self.is_zero(val_f64) {
            // Snap near-zero values (within 1e-9) to 0.0 to ensure
            // uniqueness and eliminate floating-point noise.
            OrderedFloat(0.0)
        } else {
            OrderedFloat(val_f64)
        };

        self.intern(ExprEntryKind::Number(normalized), &[])
    }

    /// Creates an addition expression: `(+ operands...)`.
    ///
    /// This operation is commutative. The underlying store will normalize
    /// the order of operands to maximize hash-consing and deduplication.
    pub fn add(&mut self, operands: &[ExprId]) -> ExprId {
        self.arithmetic(ArithmeticOp::Add, operands)
    }

    /// Creates a subtraction expression: `(- a b c ...)` which evaluates to `a - b - c`.
    ///
    /// This method ensures that all subtractions are represented in a binary or n-ary
    /// format. To maintain mathematical consistency with PDDL and other languages,
    /// it automatically handles unary negation.
    ///
    /// # Unary Negation Handling
    ///
    /// If a single operand is provided (e.g., `(- x)`), it is automatically transformed
    /// into a binary subtraction `(0.0 - x)`. This normalization allows the constant
    /// folding and flattening phases to treat negation as a standard arithmetic
    /// operation, ensuring that expressions like `(- 10)` are correctly reduced
    /// to `-10.0` during the construction pipeline.
    ///
    /// # Arguments
    ///
    /// * `operands` - A slice of [`ExprId`] representing the terms.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for the normalized subtraction.
    pub fn sub(&mut self, operands: &[ExprId]) -> ExprId {
        if operands.len() == 1 {
            // Transform unary negation into binary subtraction (0.0 - x)
            // This ensures consistent constant folding and canonical representation.
            let zero = self.number(0.0);
            self.arithmetic(ArithmeticOp::Sub, &[zero, operands[0]])
        } else {
            self.arithmetic(ArithmeticOp::Sub, operands)
        }
    }

    /// Creates a multiplication expression: `(* operands...)`.
    ///
    /// Like addition, this operation is commutative and will be normalized
    /// during the interning process.
    pub fn mul(&mut self, operands: &[ExprId]) -> ExprId {
        self.arithmetic(ArithmeticOp::Mul, operands)
    }

    /// Creates a division expression: `(/ dividend divisor)`.
    ///
    /// # Note
    ///
    /// Division is **non-commutative**. The first operand is the dividend
    /// and the second is the divisor. Any attempt to divide by a literal `0.0`
    /// will be caught by the early folding phase and return a `NaN`.
    pub fn div(&mut self, operands: &[ExprId]) -> ExprId {
        self.arithmetic(ArithmeticOp::Div, operands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::VariableId;
    use crate::aiplan4rust::lir::store::expr::ExprStore;

    /// Test: (+ 2 3) -> 5
    /// Verifies basic constant folding for addition.
    #[test]
    fn test_addition_of_constants() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);

        // Constant folding happens immediately within the builder's add method
        let root = builder.add(&[n2, n3]);

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 5.0));
    }

    /// Test: (* 2 3 4) -> 24
    /// Verifies constant folding for multiplication with multiple operands.
    #[test]
    fn test_multiplication_of_constants() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);
        let n4 = builder.number(4.0);

        let root = builder.mul(&[n2, n3, n4]);

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 24.0));
    }

    /// Test: (* x 0) -> 0
    /// Verifies the absorbing element property of zero in multiplication.
    #[test]
    fn test_multiplication_by_zero() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n0 = builder.number(0.0);

        let root = builder.mul(&[n10, n0]);

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0));
    }

    /// Test: (+ 1 (+ 2 3)) -> 6
    /// Verifies that arithmetic expressions are flattened and then folded.
    #[test]
    fn test_arithmetic_flattening() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n1 = builder.number(1.0);
        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);

        let inner_add = builder.add(&[n2, n3]); // Becomes 5.0
        let root = builder.add(&[n1, inner_add]); // (+ 1.0 5.0) -> 6.0

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 6.0));
    }

    /// Test: (/ 10 2) -> 5.0
    /// Verifies constant folding for division.
    #[test]
    fn test_division_of_constants() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n2 = builder.number(2.0);

        let root = builder.div(&[n10, n2]);

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 5.0));
    }

    /// Test: (/ 10 0) -> NaN
    /// Verifies that division by zero results in NaN as per IEEE 754.
    #[test]
    fn test_division_by_zero_returns_nan() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let tiny = builder.number(1e-12); // Snappe à 0.0

        let root = builder.div(&[n10, tiny]); // Devient 10 / 0.0

        let node = builder.get(root).expect("Node should exist");
        if let ExprEntryKind::Number(n) = node.kind() {
            assert!(
                n.into_inner().is_nan(),
                "Division by near-zero should result in NaN"
            );
        } else {
            panic!("Expected a Number(NaN) node");
        }
    }

    /// Test: (- 10 3 2) -> 5.0
    /// Verifies sequential folding for subtraction.
    #[test]
    fn test_subtraction_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n3 = builder.number(3.0);
        let n2 = builder.number(2.0);

        let root = builder.sub(&[n10, n3, n2]);

        let node = builder.get(root).expect("Node should exist");
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 5.0));
    }

    /// Test: (+ ?x 0) -> ?x
    /// Verifies that zero is a neutral element for addition with variables.
    #[test]
    fn test_addition_neutral_element() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let n0 = builder.number(0.0);

        let root = builder.add(&[var_x, n0]);

        assert_eq!(
            root, var_x,
            "Adding 0.0 should return the original operand ID"
        );
    }

    /// Test: (* ?x 1) -> ?x
    /// Verifies the neutral element of multiplication: any variable multiplied
    /// by 1.0 should return the original variable ID directly.
    #[test]
    fn test_multiplication_neutral_element() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Using a real variable ?x (id 1)
        let var_x = builder.variable(VariableId::from(1));
        let n1 = builder.number(1.0);

        let root = builder.mul(&[var_x, n1]);

        // If the builder is efficient, it should return the ID of var_x directly
        assert_eq!(
            root, var_x,
            "Multiplication by 1.0 must return the original operand"
        );
    }

    /// Test Hash-Consing: (+ 2 3) and (+ 3 2) must have the same ID
    /// Verifies that commutative operations are normalized (sorted) and folded
    /// to ensure structural uniqueness in the store.
    #[test]
    fn test_hash_consing_normalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);

        let root1 = builder.add(&[n2, n3]); // -> 5.0
        let root2 = builder.add(&[n3, n2]); // -> 5.0

        // Thanks to operand sorting and constant folding, both IDs must be identical
        assert_eq!(
            root1, root2,
            "Commutative normalization and folding should result in the same ExprId"
        );
    }

    /// Test division non-flattening: (/ 10 (/ 5 2)) != (/ 10 5 2)
    /// Verifies that unlike addition or multiplication, division is not flattened
    /// because the operation is not associative.
    #[test]
    fn test_no_division_flattening() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n5 = builder.number(5.0);
        let n2 = builder.number(2.0);

        let inner_div = builder.div(&[n5, n2]); // 2.5
        let root = builder.div(&[n10, inner_div]); // 10 / 2.5 = 4.0

        let node = builder.get(root).expect("Root node should exist");

        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 4.0),
            "Division folding should respect nested structure and return 4.0"
        );
    }

    /// Test: (+ 10 ?x 2) -> (+ 12 ?x)
    /// Verifies that constant folding "jumps" over variables in commutative
    /// operations to collect and aggregate all numeric literals.
    #[test]
    fn test_mixed_folding_commutative() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Prepare operands
        let n10 = builder.number(10.0);
        let n2 = builder.number(2.0);
        let var_x = builder.variable(VariableId::from(1));

        // Operation: (+ 10 ?x 2)
        let root = builder.add(&[n10, var_x, n2]);

        let node = builder.get(root).expect("Root node should exist");

        // We expect exactly 2 children: the variable ?x and the folded constant 12.0
        assert_eq!(
            node.children().len(),
            2,
            "The expression should be reduced to exactly 2 operands"
        );
        assert!(
            node.children().contains(&var_x),
            "The symbolic variable ?x must be preserved"
        );

        // Find the child node that is not the variable to verify the folded constant
        let const_child_id = *node
            .children()
            .iter()
            .find(|&&id| id != var_x)
            .expect("Should contain a constant child node");

        let const_node = builder
            .get(const_child_id)
            .expect("Constant child node should exist");

        assert!(
            matches!(const_node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 12.0),
            "The constant part should be 12.0 (folded from 10 + 2)"
        );
    }

    /// Test: (* ?x ?y 0) -> 0.0
    /// Verifies that zero acts as an absorbing element for multiplication,
    /// effectively "wiping out" any number of symbolic variables.
    #[test]
    fn test_absorbant_with_variables() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create two distinct variables
        let var_x = builder.variable(VariableId::from(1));
        let var_y = builder.variable(VariableId::from(2));
        let n0 = builder.number(0.0);

        // Operation: (* ?x ?y 0.0)
        let root = builder.mul(&[var_x, var_y, n0]);

        // The folding logic should return the 0.0 node ID directly.
        let node = builder.get(root).expect("Resulting node should exist");

        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Multiplication by zero must absorb all variables and return 0.0"
        );
    }

    /// Test: (/ 1 (/ 0 0)) -> NaN
    /// Verifies that NaN propagates through nested operations.
    /// Any operation with a NaN operand should be simplified to a constant NaN.
    #[test]
    fn test_nan_propagation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n0 = builder.number(0.0);
        let n1 = builder.number(1.0);

        // div_zero = (0 / 0) -> NaN
        let div_zero = builder.div(&[n0, n0]);

        // root = (1 / NaN) -> should be folded to NaN
        let root = builder.div(&[n1, div_zero]);

        let node = builder.get(root).expect("Root node should exist");

        if let ExprEntryKind::Number(n) = node.kind() {
            assert!(
                n.into_inner().is_nan(),
                "The result of any operation involving NaN should be NaN"
            );
        } else {
            panic!("Expected a Number(NaN) node, but found an expression node instead");
        }
    }

    /// Test: ((?x + 1.0) * 2.0) == (2.0 * (1.0 + ?x))
    /// Verifies deep Hash-Consing and recursive normalization.
    /// Even with different construction orders and nesting, commutative
    /// operations must result in the exact same ExprId.
    #[test]
    fn test_deep_hash_consing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Define variable ?x
        let var_x = builder.variable(VariableId::from(1));

        // --- Construction of Version A: ((var_x + 1.0) * 2.0) ---
        let n1_a = builder.number(1.0);
        let add1 = builder.add(&[var_x, n1_a]);
        let n2_a = builder.number(2.0);
        let expr1 = builder.mul(&[add1, n2_a]);

        // --- Construction of Version B: (2.0 * (1.0 + var_x)) ---
        // We flip the operand order at every level to challenge the normalizer.
        let n2_b = builder.number(2.0);
        let n1_b = builder.number(1.0);
        let add2 = builder.add(&[n1_b, var_x]); // Flipped order: (1.0 + ?x)
        let expr2 = builder.mul(&[n2_b, add2]); // Flipped order: (2.0 * add)

        // Hash-Consing combined with internal sorting must guarantee
        // that both paths lead to the same unique identifier.
        assert_eq!(
            expr1, expr2,
            "Deep hash-consing and normalization should unify these structurally different paths"
        );
    }

    /// Test: (- ?x 0) -> ?x
    /// Verifies the right neutral element of subtraction: subtracting 0.0
    /// from any variable should return the original variable ID.
    #[test]
    fn test_subtraction_neutral_right() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let n0 = builder.number(0.0);

        // Operation: x - 0.0
        let root = builder.sub(&[var_x, n0]);

        // The builder should simplify this identity (x - 0 = x) and avoid
        // creating an unnecessary expression node in the store.
        assert_eq!(
            root, var_x,
            "Subtracting 0.0 from a variable must return the original ID"
        );
    }

    /// Test: (- 0 ?x) -> (- 0 ?x)
    /// Verifies that zero is NOT a left-neutral element for subtraction.
    /// Unlike (x - 0), which simplifies to x, (0 - x) must remain a valid
    /// subtraction expression (representing negation).
    #[test]
    fn test_subtraction_non_neutral_left() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n0 = builder.number(0.0);
        let var_x = builder.variable(VariableId::from(1));

        // Operation: 0.0 - x
        let root = builder.sub(&[n0, var_x]);

        // 0 - x is not equal to x, so the ID must be different from var_x.
        assert_ne!(root, var_x, "0 - x should not be simplified to x");

        let node = builder.get(root).expect("Resulting node should exist");

        // We verify that the expression still has both operands.
        assert_eq!(
            node.children().len(),
            2,
            "The subtraction node (0 - x) should contain two children"
        );
        assert_eq!(node.children()[0], n0);
        assert_eq!(node.children()[1], var_x);
    }

    /// Test: (/ ?x 1) -> ?x
    /// Verifies the right neutral element of division: any variable divided
    /// by 1.0 should return the original variable ID directly.
    #[test]
    fn test_division_neutral_right() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let n1 = builder.number(1.0);

        // Operation: x / 1.0
        let root = builder.div(&[var_x, n1]);

        // The builder should detect the identity (x / 1 = x) and return
        // the ID of var_x without creating a new expression node.
        assert_eq!(
            root, var_x,
            "Dividing by 1.0 must return the original operand"
        );
    }

    /// Test: (- 10 ?x 2) -> (- 10 ?x 2) (Strict Order)
    /// Verifies that the folder does not simplify constants that are separated
    /// by a variable in non-commutative operations.
    ///
    /// Folding (- 10 ?x 2) into (- 8 ?x) would be mathematically incorrect
    /// because (10 - x - 2) is not necessarily equal to (8 - x) in all
    /// arithmetic contexts (though it is in standard real math, LIR folders
    /// usually stay conservative to avoid unexpected side effects).
    #[test]
    fn test_subtraction_order_safety() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let var_x = builder.variable(VariableId::from(1));
        let n2 = builder.number(2.0);

        let root = builder.sub(&[n10, var_x, n2]);
        let node = builder.get(root).expect("Root node should exist");

        // We verify that all 3 operands remain in their original order.
        // The folding must stop as soon as it encounters var_x at index 1.
        assert_eq!(
            node.children().len(),
            3,
            "The folder should not have merged constants across the variable"
        );
        assert_eq!(node.children()[0], n10);
        assert_eq!(node.children()[1], var_x);
        assert_eq!(node.children()[2], n2);
    }

    /// Test: (- 10 2 ?x 3) -> (- 8 ?x 3)
    /// Verifies that "head folding" works for non-commutative operations:
    /// Initial constants are folded, but the process stops as soon as a variable
    /// is encountered to preserve mathematical order.
    #[test]
    fn test_subtraction_head_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n2 = builder.number(2.0);
        let var_x = builder.variable(VariableId::from(1));
        let n3 = builder.number(3.0);

        // Operation: 10 - 2 - x - 3
        let root = builder.sub(&[n10, n2, var_x, n3]);
        let node = builder.get(root).expect("Root node should exist");

        // We expect 3 operands: 8.0 (result of 10-2), var_x, and 3.0
        assert_eq!(
            node.children().len(),
            3,
            "Should have exactly 3 operands after head-folding"
        );

        let first_child = builder
            .get(node.children()[0])
            .expect("First child should exist");
        assert!(
            matches!(first_child.kind(), ExprEntryKind::Number(n) if n.into_inner() == 8.0),
            "The head constants (10, 2) should be folded into 8.0"
        );

        assert_eq!(
            node.children()[1],
            var_x,
            "The second operand must be the variable ?x"
        );
        assert_eq!(
            node.children()[2],
            n3,
            "The constant 3.0 must remain separate as it follows a variable"
        );
    }

    /// Test: (+ -5 5) -> 0.0
    /// Verifies that constant folding correctly handles negative numbers and
    /// results in a zero constant when they cancel out.
    #[test]
    fn test_negative_numbers_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n_neg_5 = builder.number(-5.0);
        let n_pos_5 = builder.number(5.0);

        let root = builder.add(&[n_neg_5, n_pos_5]);
        let node = builder.get(root).expect("Resulting node should exist");

        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Adding -5.0 and 5.0 should be folded to 0.0"
        );
    }

    /// Test: (+ 1 (+ ?x 2) 3) -> (+ 6 ?x)
    /// The ultimate combined test: Recursive Flattening + Mixed Constant Folding +
    /// Commutative Normalization + Hash-Consing.
    #[test]
    fn test_mega_unification() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));

        // --- Construction of Version A: (+ 1 (+ ?x 2) 3) ---
        let n2 = builder.number(2.0);
        let inner_a = builder.add(&[var_x, n2]);

        let n1 = builder.number(1.0);
        let n3 = builder.number(3.0);

        // The builder should:
        // 1. Flatten inner_a: (+ 1 ?x 2 3)
        // 2. Fold constants: (+ ?x 6.0)
        // 3. Sort/Normalize: (+ 6.0 ?x)
        let root_a = builder.add(&[n1, inner_a, n3]);

        // --- Construction of Version B: (+ 6 ?x) ---
        let n6 = builder.number(6.0);
        let root_b = builder.add(&[n6, var_x]);

        // Thanks to Hash-Consing and normalization, both IDs must be identical.
        assert_eq!(
            root_a, root_b,
            "The complex expression (1 + (x + 2) + 3) should be unified to (6 + x)"
        );
    }

    /// Test: (* ?x (/ 0 0)) -> NaN
    /// Verifies that NaN acts as an absorbing element and is propagated,
    /// even when combined with symbolic variables.
    #[test]
    fn test_nan_absorption() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create variable ?x
        let var_x = builder.variable(VariableId::from(1));

        // 2. Create NaN via 0.0 / 0.0
        let n0 = builder.number(0.0);
        let nan_val = builder.div(&[n0, n0]);

        // 3. Multiply the variable by NaN
        // The constant folder should ideally reduce this to a single NaN constant.
        let root = builder.mul(&[var_x, nan_val]);
        let node = builder.get(root).expect("Root node should exist");

        // 4. Verification
        if let ExprEntryKind::Number(n) = node.kind() {
            // Successful constant folding: the expression became a literal NaN.
            assert!(
                n.into_inner().is_nan(),
                "The result of mul(x, NaN) should be reduced to NaN"
            );
        } else {
            // Fallback: if folding isn't aggressive enough, NaN must still be present in operands.
            assert!(
                node.children().contains(&nan_val),
                "If the expression is not fully reduced, NaN must remain in the children"
            );
        }
    }

    /// Test: (/ 0 ?x) -> 0.0
    /// Verifies that zero divided by any symbolic variable is simplified to 0.0.
    ///
    /// # Note
    /// While mathematically risky if ?x evaluates to 0 at runtime (which should yield NaN),
    /// this simplification is standard in many LIR implementations to keep expressions lean.
    #[test]
    fn test_division_by_zero_numerator() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n0 = builder.number(0.0);
        let var_x = builder.variable(VariableId::from(1));

        let root = builder.div(&[n0, var_x]);

        let node = builder.get(root).expect("Resulting node should exist");

        // Verifies that the builder optimizes 0/x into a constant 0.0
        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Division with a zero numerator should be simplified to 0.0"
        );
    }

    /// Test: (* 2 (* 3 ?x) 4) -> (* 24 ?x)
    /// Verifies that nested multiplications are flattened and constants are
    /// aggregated even when separated by a variable in the original structure.
    #[test]
    fn test_deep_flattening_multiplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let n3 = builder.number(3.0);
        let n2 = builder.number(2.0);
        let n4 = builder.number(4.0);

        // inner_mul = (* 3 x)
        let inner_mul = builder.mul(&[n3, var_x]);
        // root = (* 2 (* 3 x) 4) -> should flatten to (* 24 x)
        let root = builder.mul(&[n2, inner_mul, n4]);

        let node_ref = builder.get(root).expect("Root node should exist");

        // After flattening and folding, we expect exactly two operands: the variable and the constant.
        assert_eq!(
            node_ref.children().len(),
            2,
            "Expression should be reduced to 2 operands"
        );

        // Find the constant operand (the one that isn't the variable)
        let const_child_id = *node_ref
            .children()
            .iter()
            .find(|&&id| id != var_x)
            .expect("Should contain a constant child node");

        let const_node = builder
            .get(const_child_id)
            .expect("Constant child node should exist");
        let val = const_node.kind();

        assert!(
            matches!(val, ExprEntryKind::Number(n) if n.into_inner() == 24.0),
            "The constant part should be 24.0 (2 * 3 * 4)"
        );
    }

    /// Test: (- ?x ?x) -> 0.0
    /// Verifies if the builder simplifies a variable subtracted from itself.
    #[test]
    fn test_subtraction_self_identity() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));

        let root = builder.sub(&[var_x, var_x]);

        let node = builder.get(root).expect("Resulting node should exist");

        // This confirms that symbolic identity (x - x = 0) is correctly
        // handled during the building/folding phase.
        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Subtracting a variable from itself should result in 0.0"
        );
    }

    /// Test: (/ ?x ?x) -> 1.0
    #[test]
    fn test_division_self_identity() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));

        let root = builder.div(&[var_x, var_x]);

        let node = builder.get(root).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 1.0));
    }

    /// Test: (- ?x ?y ?z)
    /// Ensures that non-commutative operations preserve order for multiple variables.
    #[test]
    fn test_subtraction_multiple_variables() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let var_y = builder.variable(VariableId::from(2));
        let var_z = builder.variable(VariableId::from(3));

        let root = builder.sub(&[var_x, var_y, var_z]);
        let node = builder.get(root).unwrap();

        assert_eq!(node.children().len(), 3);
        assert_eq!(node.children()[0], var_x);
        assert_eq!(node.children()[1], var_y);
        assert_eq!(node.children()[2], var_z);
    }

    /// Test: (- 10 2 ?x 3 1) -> (- 8 ?x 3 1)
    /// Checks that folding only happens at the head and constants after variables are preserved.
    #[test]
    fn test_subtraction_complex_mixed_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let n10 = builder.number(10.0);
        let n2 = builder.number(2.0);
        let var_x = builder.variable(VariableId::from(1));
        let n3 = builder.number(3.0);
        let n1 = builder.number(1.0);

        let root = builder.sub(&[n10, n2, var_x, n3, n1]);
        let node = builder.get(root).unwrap();

        // Expected: [8.0, var_x, 3.0, 1.0]
        assert_eq!(node.children().len(), 4);
        let first_val = builder.get_number(node.children()[0]).unwrap();
        assert_eq!(first_val, 8.0);
        assert_eq!(node.children()[1], var_x);
        assert_eq!(node.children()[2], n3);
        assert_eq!(node.children()[3], n1);
    }

    /// Test: (- (+ ?x 1) (+ ?x 1)) -> 0.0
    /// Verifies that symbolic identities work on complex expressions thanks to interning.
    #[test]
    fn test_complex_self_identity() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::from(1));
        let n1 = builder.number(1.0);

        let complex_op = builder.add(&[var_x, n1]);

        // Construction: (- complex_op complex_op)
        let root = builder.sub(&[complex_op, complex_op]);

        let node = builder.get(root).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0));
    }

    /// Test: (+ x 1e-12) -> x
    /// Verifies that "noisy" neutral elements (near zero) are eliminated during folding.
    #[test]
    fn test_arithmetic_epsilon_neutral_addition() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x = builder.variable(VariableId::new(1));
        let tiny = builder.number(1e-12); // Snappe à 0.0 ici

        let root = builder.add(&[x, tiny]);

        assert_eq!(root, x, "Addition with near-zero should be simplified to x");
    }

    /// Test: (* x 0.999999999999) -> x
    /// Verifies that multiplication by a value near 1.0 is simplified.
    #[test]
    fn test_arithmetic_epsilon_neutral_multiplication() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x = builder.variable(VariableId::new(1));
        let near_one = builder.number(1.0 - 1e-12); // Proche de 1.0

        let root = builder.mul(&[x, near_one]);

        assert_eq!(
            root, x,
            "Multiplication by near-one should be simplified to x"
        );
    }

    /// Test: (* x 1e-12) -> 0.0
    /// Verifies that multiplication by a near-zero value absorbs the expression.
    #[test]
    fn test_arithmetic_epsilon_absorption() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x = builder.variable(VariableId::new(1));
        let near_zero = builder.number(1e-12);

        let root = builder.mul(&[x, near_zero]);

        let node = builder.fetch(root).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0));
    }

    /// Test: (- x x) -> 0.0
    /// Verifies symbolic identity (x - x) enabled by Hash-Consing.
    #[test]
    fn test_symbolic_identity_subtraction() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x = builder.variable(VariableId::new(1));
        let root = builder.sub(&[x, x]);

        let node = builder.fetch(root).unwrap();
        assert!(matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0));
    }

    /// Verifies that accumulated noise below epsilon is discarded when a variable
    /// is present, preventing expression pollution.
    #[test]
    fn test_constant_folding_accumulation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let tiny = builder.number(1e-11);
        let ops = vec![tiny; 10]; // Somme de 10 * 1e-11 = 1e-10

        let root = builder.add(&ops);
        let node = builder.fetch(root).unwrap();

        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Expected 0.0, found {:?}",
            node.kind()
        );
    }

    /// Verifies that pure constant noise (no variables) results in exactly 0.0.
    #[test]
    fn test_constant_folding_pure_noise() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let tiny = builder.number(1e-11);
        let ops = vec![tiny; 10]; // 10 * 1e-11 = 1e-10

        let root = builder.add(&ops);

        // Fetch and verify in a single match arm
        let node = builder.fetch(root).unwrap();
        assert!(
            matches!(node.kind(), ExprEntryKind::Number(n) if n.into_inner() == 0.0),
            "Expected 0.0, found {:?}",
            node.kind()
        );
    }

    /// Test: (+ x y) == (+ y x)
    /// Verifies that commutative operations are sorted to ensure unique Hash-Consing.
    #[test]
    fn test_commutative_canonical_sorting() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x = builder.variable(VariableId::new(1));
        let y = builder.variable(VariableId::new(2));

        let sum1 = builder.add(&[x, y]);
        let sum2 = builder.add(&[y, x]);

        assert_eq!(
            sum1, sum2,
            "Commutative operands must be sorted to produce identical IDs"
        );
    }

    /// Test: Hash-Consing & Epsilon
    /// Verifies that the interning mechanism (Hash-Consing) unifies constants
    /// considered zero-equivalent according to the epsilon (1e-9).
    ///
    /// Without "snapping" inside `builder.number()`, 0.0 and 1e-15 would have
    /// different IDs, breaking structural $O(1)$ equality.
    #[test]
    fn test_epsilon_hash_consing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create a pure zero
        let n_zero = builder.number(0.0);

        // Create a value well below the epsilon (1e-15 < 1e-9)
        let n_tiny = builder.number(1e-15);

        // Both must return exactly the same ExprId.
        // This ensures that (+ x 0.0) and (+ x 1e-15) are simplified
        // to the same state, optimizing memory and future comparisons.
        assert_eq!(
            n_zero, n_tiny,
            "0.0 and 1e-15 must be unified to the same ExprId by the epsilon-aware builder"
        );
    }
}
