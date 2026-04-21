use crate::aiplan4rust::lang::ArithmeticOp;
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};
use ordered_float::OrderedFloat;
use smallvec::SmallVec;

/// The threshold for stack-based operand storage.
///
/// Expressions with fewer than 16 operands are handled without heap allocation,
/// which covers the vast majority of PDDL expressions and significantly improves
/// performance by reducing pressure on the allocator and improving cache locality.
const INITIAL_OPERAND_STACK_CAPACITY: usize = 16;

/// A specialized vector for expression IDs, optimized for small amounts of operands.
///
/// It stays on the stack if the number of IDs is <= `INITIAL_OPERAND_STACK_CAPACITY`,
/// and transparently spills to the heap if the expression is larger.
type OperandStack = SmallVec<[ExprId; INITIAL_OPERAND_STACK_CAPACITY]>;

impl<'a> ExprBuilder<'a> {
    /// Constructs an optimized arithmetic expression: `(op operands...)`.
    ///
    /// This function orchestrates a multi-step pipeline to ensure the resulting
    /// [`ExprId`] points to the most simplified and canonical form of the expression.
    ///
    /// # Pipeline Steps
    ///
    /// 1. **Early Fold**: Immediate resolution of trivial cases like `NaN`, division by zero,
    ///    or symbolic identities (e.g., `x - x`).
    /// 2. **Flatten**: Structural normalization of associative operations (`Add`, `Mul`).
    /// 3. **Constant Folding**: Numerical reduction of all literal constants.
    /// 4. **Finalize**: Sorting of commutative operands and interning into the store.
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
        // --- PHASE 1: EARLY FOLDING ---
        // Intercept trivial or propagative cases before any heavy processing.
        if let Some(id) = self.early_fold_nan(operands) {
            return id;
        }
        if let Some(id) = self.early_fold_div_by_zero(op, operands) {
            return id;
        }
        if let Some(id) = self.early_fold_identities(op, operands) {
            return id;
        }

        // --- PHASE 2: FLATTENING ---
        // Structural transformation: `(+ (+ a b) c) => (+ a b c)`
        let flat_ops = self.flatten_operands(op, operands);

        // --- PHASE 3: CONSTANT FOLDING ---
        // Aggregate all numerical constants into a single value.
        let mut fold_ops = OperandStack::with_capacity(flat_ops.len());
        if let Some(reduced_id) = self.fold_constants(op, &flat_ops, &mut fold_ops) {
            return reduced_id;
        }

        // --- PHASE 4: FINALIZATION ---
        // Sort commutative operands and intern the final result.
        self.finalize(op, fold_ops)
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
    /// If the operation is a division and the divisor is a literal `0.0`,
    /// it immediately returns a `NaN` constant to prevent invalid runtime calculations.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] being performed.
    /// * `operands` - A slice of [`ExprId`] containing the dividend and the divisor.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - An ID pointing to a `NaN` constant if a division by zero is detected.
    /// * `None` - If no division by zero is found, allowing the pipeline to proceed.
    fn early_fold_div_by_zero(&mut self, op: ArithmeticOp, operands: &[ExprId]) -> Option<ExprId> {
        if op == ArithmeticOp::Div && operands.len() == 2 {
            // We only check the second operand (the divisor)
            let divisor_id = operands[1];
            if let Some(node) = self.get(divisor_id) {
                if let ExprEntryKind::Number(n) = node.kind() {
                    if n.into_inner() == 0.0 {
                        return Some(self.number(f64::NAN));
                    }
                }
            }
        }
        None
    }

    /// Simplifies symbolic identities where the result is known regardless of the variable's value.
    ///
    /// This handles cases like `x - x` which always equals `0.0`, and `x / x` which
    /// always equals `1.0` (assuming `x` is not zero, though PDDL often simplifies this).
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] to check for identity simplification.
    /// * `operands` - A slice of [`ExprId`] representing the two operands.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - An ID pointing to the simplified constant (`0.0` or `1.0`).
    /// * `None` - If no symbolic identity is found.
    fn early_fold_identities(&mut self, op: ArithmeticOp, operands: &[ExprId]) -> Option<ExprId> {
        if operands.len() == 2 && operands[0] == operands[1] {
            match op {
                ArithmeticOp::Sub => return Some(self.number(0.0)),
                ArithmeticOp::Div => return Some(self.number(1.0)),
                _ => {}
            }
        }
        None
    }

    /// Parcourt les opérandes et les aplatit si ce sont des opérations identiques
    /// et associatives (Add ou Mul).
    fn flatten_operands(&self, op: ArithmeticOp, operands: &[ExprId]) -> OperandStack {
        // 1. Si pas associatif, on rend les opérandes tels quels dans la SmallVec
        if !matches!(op, ArithmeticOp::Add | ArithmeticOp::Mul) {
            return OperandStack::from_slice(operands);
        }

        // 2. On vérifie s'il y a vraiment besoin d'aplatir
        let needs_flattening = operands.iter().any(|&id| {
            self.get(id).map_or(false, |node| {
                matches!(node.kind(), ExprEntryKind::Arithmetic(child_op) if *child_op == op)
            })
        });

        // 3. Si c'est déjà plat, on évite la logique complexe
        if !needs_flattening {
            return OperandStack::from_slice(operands);
        }

        // 4. Aplatissement réel
        let mut flat = OperandStack::with_capacity(operands.len());
        for &id in operands {
            if let Some(node) = self.get(id) {
                if let ExprEntryKind::Arithmetic(child_op) = node.kind() {
                    if *child_op == op {
                        flat.extend_from_slice(node.children());
                        continue;
                    }
                }
            }
            flat.push(id);
        }
        flat
    }

    /// Numerically reduces constants within a flattened list of operands.
    ///
    /// This function performs a linear pass over the operands to aggregate literal numbers.
    /// It handles propagative cases (like `NaN` or `0 * x`) and delegates specific algebraic
    /// rules to `apply_folding_rule`. Operands that cannot be reduced (variables) are
    /// pushed to the `final_ops` stack.
    ///
    /// # Workflow
    /// 1. **Extraction**: Attempts to resolve each [`ExprId`] into a concrete `f64`.
    /// 2. **Short-circuiting**: Immediately returns if a `NaN` is found or if a
    ///    multiplication by zero occurs.
    /// 3. **Folding**: Applies operator-specific rules to accumulate constants.
    /// 4. **Variable Tracking**: Flags when a variable is encountered to prevent
    ///    incorrect folding in non-commutative operations (`Sub`, `Div`).
    /// 5. **Reinsertion**: After the loop, any accumulated constant is placed back
    ///    into the operand stack.
    ///
    /// # Arguments
    ///
    /// * `op` - The current [`ArithmeticOp`] (Add, Sub, Mul, Div).
    /// * `flat_ops` - A slice of [`ExprId`] that has already been structurally flattened.
    /// * `final_ops` - A mutable reference to the [`OperandStack`] where non-reducible
    ///   operands are collected.
    ///
    /// # Returns
    ///
    /// * `Some(ExprId)` - If the expression is reduced to a single value via short-circuit
    ///   (e.g., `0.0` for multiplication or an existing `NaN`).
    /// * `None` - If the folding process completed normally (even if no reduction was possible).
    /// Numerically reduces constants within the expression.
    ///
    /// This function performs a linear pass to aggregate literal numbers. It handles
    /// algebraic rules directly to avoid excessive parameter passing while maintaining
    /// precise control over non-commutative operations.
    fn fold_constants(
        &mut self,
        op: ArithmeticOp,
        flat_ops: &[ExprId],
        fold_ops: &mut OperandStack,
    ) -> Option<ExprId> {
        let mut constant_part: Option<f64> = None;
        let mut has_variable = false;

        for (i, &id) in flat_ops.iter().enumerate() {
            match self.get_number(id) {
                Some(v) => {
                    // --- CASE: NUMBER ---
                    if v.is_nan() {
                        return Some(id);
                    }
                    if op == ArithmeticOp::Mul && v == 0.0 {
                        return Some(self.number(0.0));
                    }

                    match (op, constant_part) {
                        // Commutative: aggregate everything
                        (ArithmeticOp::Add, c) => constant_part = Some(c.unwrap_or(0.0) + v),
                        (ArithmeticOp::Mul, c) => constant_part = Some(c.unwrap_or(1.0) * v),

                        // Positional (Sub/Div): set the base if it's the first element
                        (ArithmeticOp::Sub | ArithmeticOp::Div, None) if i == 0 => {
                            constant_part = Some(v);
                        }

                        // Sequential folding: only before any variable appears
                        (ArithmeticOp::Sub, Some(c)) if !has_variable => {
                            constant_part = Some(c - v);
                        }
                        (ArithmeticOp::Div, Some(c)) if !has_variable => {
                            if v == 0.0 {
                                return Some(self.number(f64::NAN));
                            }
                            constant_part = Some(c / v);
                        }

                        // Identity: skip x - 0 or x / 1
                        (ArithmeticOp::Sub, _) if v == 0.0 => {}
                        (ArithmeticOp::Div, _) if v == 1.0 => {}

                        // Fallback: cannot fold (e.g., after a variable)
                        _ => fold_ops.push(id),
                    }
                }
                None => {
                    // --- CASE: VARIABLE ---
                    fold_ops.push(id);
                    has_variable = true;

                    // Optimization: 0 / variable -> 0
                    if op == ArithmeticOp::Div && constant_part == Some(0.0) {
                        return Some(self.number(0.0));
                    }
                }
            }
        }

        self.reinsert_constant(op, constant_part, fold_ops);
        None
    }

    /// Extracts a literal floating-point value from an expression ID if it points to a number.
    ///
    /// This is a convenience helper that traverses the `ExprStore` to check if a specific
    /// [`ExprId`] corresponds to a [`ExprEntryKind::Number`].
    ///
    /// # Arguments
    ///
    /// * `id` - The [`ExprId`] of the expression to inspect.
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The inner value if the expression is a numeric literal.
    /// * `None` - If the expression does not exist or is not a number (e.g., it's a variable or another operation).
    fn get_number(&self, id: ExprId) -> Option<f64> {
        self.get(id).and_then(|n| {
            if let ExprEntryKind::Number(v) = n.kind() {
                Some(v.into_inner())
            } else {
                None
            }
        })
    }

    /// Re-inserts the accumulated constant into the operand stack at the correct position.
    ///
    /// This is the final step of the constant folding process. It decides whether the
    /// accumulated constant is significant enough to be added to the expression
    /// or if it can be omitted (in the case of neutral elements).
    ///
    /// # Positioning Logic
    ///
    /// * **Commutative (Add, Mul)**: The constant is simply pushed to the end of the stack.
    /// * **Positional (Sub, Div)**: The constant represents the base value (the minuend
    ///   or dividend) and must be inserted at the very beginning (index 0) to maintain
    ///   the correct order of operations (e.g., `10 - x - y`).
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] currently being finalized.
    /// * `constant_part` - The optional accumulated `f64` result from the folding phase.
    /// * `final_ops` - A mutable reference to the [`OperandStack`] where the constant will be inserted.
    fn reinsert_constant(
        &mut self,
        op: ArithmeticOp,
        constant_part: Option<f64>,
        final_ops: &mut OperandStack,
    ) {
        if let Some(c) = constant_part {
            // Check if the constant is a neutral element (0.0 for Add, 1.0 for Mul).
            // Neutral elements can be discarded unless the expression would be empty.
            let is_neutral = match op {
                ArithmeticOp::Add => c == 0.0,
                ArithmeticOp::Mul => c == 1.0,
                _ => false,
            };

            // We re-insert if:
            // 1. The value is not neutral (it changes the result).
            // 2. The stack is empty (we need at least one value, even if neutral).
            if !is_neutral || final_ops.is_empty() {
                let const_id = self.number(c);
                if matches!(op, ArithmeticOp::Add | ArithmeticOp::Mul) {
                    final_ops.push(const_id);
                } else {
                    // For Sub/Div, the constant part always represents the starting value.
                    final_ops.insert(0, const_id);
                }
            }
        }
    }

    /// Finalizes the arithmetic expression by normalizing operands and interning the result.
    ///
    /// This function handles the last stage of the construction pipeline:
    /// 1. **Identity Resolution**: Returns a neutral constant if no operands remain.
    /// 2. **Simplification**: Returns the single operand directly if no operation is needed.
    /// 3. **Canonicalization**: Sorts operands for commutative operations (`Add`, `Mul`)
    ///    to ensure that different orderings result in the same [`ExprId`].
    /// 4. **Interning**: Deduplicates the final expression in the store.
    ///
    /// # Arguments
    ///
    /// * `op` - The [`ArithmeticOp`] of the expression.
    /// * `ops` - The [`OperandStack`] containing the final, folded operands.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for this expression.
    fn finalize(&mut self, op: ArithmeticOp, mut ops: OperandStack) -> ExprId {
        // 1. Empty case: Return neutral element for the given operation.
        // e.g., (+) -> 0.0, (*) -> 1.0
        if ops.is_empty() {
            return self.number(match op {
                ArithmeticOp::Add | ArithmeticOp::Sub => 0.0,
                ArithmeticOp::Mul | ArithmeticOp::Div => 1.0,
            });
        }

        // 2. Single operand case: (+ x) is just x.
        if ops.len() == 1 {
            return ops[0];
        }

        // 3. Normalization for interning:
        // By sorting commutative operations, we ensure that (+ a b) and (+ b a)
        // are recognized as the same structural expression, maximizing cache hits.
        if matches!(op, ArithmeticOp::Add | ArithmeticOp::Mul) {
            ops.sort_unstable();
        }

        // 4. Final interning: Create or retrieve the ID from the store.
        self.intern(ExprEntryKind::Arithmetic(op), &ops)
    }

    /// Creates a numeric literal node with mandatory NaN normalization.
    ///
    /// To ensure perfect deduplication (hash-consing), this function normalizes all
    /// variations of `NaN` into a single canonical representation. Without this,
    /// different bit patterns of `NaN` (e.g., quiet vs. signaling) would result
    /// in different [`ExprId`]s, breaking the store's invariants.
    ///
    /// # Performance Note
    ///
    /// The normalization check (`is_nan`) is extremely cheap (a simple bitmask
    /// on the exponent) and is preferred here rather than inside the generic `intern`
    /// method to keep the main interning loop as fast as possible.
    ///
    /// # Arguments
    ///
    /// * `value` - The floating-point value to wrap and normalize.
    ///
    /// # Returns
    ///
    /// * `ExprId` - The unique identifier for this numeric constant.
    pub fn number(&mut self, value: f64) -> ExprId {
        let val = if value.is_nan() {
            // Normalize to a single canonical NaN representation
            OrderedFloat(f64::NAN)
        } else {
            OrderedFloat(value)
        };
        self.intern(ExprEntryKind::Number(val), &[])
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
    /// # Note
    ///
    /// Subtraction is **non-commutative**. The order of operands is strictly
    /// preserved to maintain mathematical correctness.
    pub fn sub(&mut self, operands: &[ExprId]) -> ExprId {
        self.arithmetic(ArithmeticOp::Sub, operands)
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
    use crate::aiplan4rust::lir::store::ExprStore;

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
        let n0 = builder.number(0.0);

        let root = builder.div(&[n10, n0]);

        let node = builder.get(root).expect("Node should exist");
        if let ExprEntryKind::Number(n) = node.kind() {
            assert!(n.into_inner().is_nan(), "Should result in a NaN constant");
        } else {
            panic!("Expected a Number(NaN) kind");
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
}
