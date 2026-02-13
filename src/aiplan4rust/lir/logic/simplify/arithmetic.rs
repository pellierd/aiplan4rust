use ordered_float::OrderedFloat;
use crate::aiplan4rust::lang::ArithmeticOp;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::tree::{NodeId, SyntaxContent};

/// Simplifies an arithmetic expression node.
///
/// This function centralizes the arithmetic simplification process, ensuring that
/// arithmetic expr are normalized and constants are evaluated. It performs
/// two main steps:
///
/// 1. **Normalization (flattening)**: Nested arithmetic operations of the same kind
///    (`+` or `*`) are flattened into a single node. For example:
///    - `(+ 1 (+ 2 3) 4)` → `(+ 1 2 3 4)`
///    - `(* 2 (* 3 4))` → `(* 2 3 4)`
///
/// 2. **Reduction (constant evaluation)**: If all children of the operation are
///    constants (`Number` nodes), the operation is evaluated and replaced by a
///    single constant node.
///
/// # Parameters
///
/// * `node_id` - The ID of the arithmetic operation node to simplify.
/// * `expr` - The mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(())` if simplification succeeds or is not applicable.
/// * `Err(ExprError)` if an arithmetic evaluation fails (e.g., division by zero).
///
/// # Notes
///
/// - Only arithmetic operation nodes (`ExprKind::Operation`) are affected.
/// - Non-arithmetic nodes are skipped silently.
/// - Flattening and constant evaluation are applied sequentially, preserving
///   the tree structure while simplifying expr.
pub fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), LogicError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::Operation {
        return Ok(());
    }

    // Step 1: Flatten nested operations of the same type
    flatten_arithmetic_expression(node_id, expr)?;

    // Step 2: Evaluate constants
    reduce(node_id, expr)?;

    Ok(())
}

/// Flattens nested arithmetic expr of the same operator.
///
/// This function normalizes arithmetic expr by pulling up children from
/// nested operations of the same kind (`+` or `*`). It ensures that expr
/// like nested sums or products are represented in a flat, normalized form.
///
/// # Examples
///
/// - `(+ 1 (+ 2 3) 4)` → `(+ 1 2 3 4)`
/// - `(* 2 (* 3 4))`   → `(* 2 3 4)`
///
/// # Parameters
///
/// * `node_id` - The ID of the arithmetic operation node to types.
/// * `expr` - The mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(())` if flattening succeeds or is not applicable.
/// * `Err(ExprError)` if node access fails.
///
/// # Notes
///
/// - Only arithmetic operation nodes (`ExprKind::Operation`) are affected.
/// - Non-arithmetic nodes are skipped silently.
/// - This function does not evaluate constants; it only normalizes the tree structure.
fn flatten_arithmetic_expression(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), LogicError> {
    let node = expr.try_node(node_id)?;

    // Only arithmetic operation nodes can be flattened
    if node.kind() != ExprKind::Operation {
        return Ok(());
    }

    let op = match node.content().as_arithmetic_op() {
        Some(op) => op,
        None => return Ok(()),
    };

    let mut new_children: Vec<NodeId> = Vec::with_capacity(node.children().len());

    for &child_id in node.children() {
        let child = expr.try_node(child_id)?;

        // If child is the same arithmetic operation, pull up its children
        if child.kind() == ExprKind::Operation {
            if let Some(child_op) = child.content().as_arithmetic_op() {
                if child_op == op {
                    new_children.extend(child.children());
                    continue;
                }
            }
        }

        new_children.push(child_id);
    }

    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_children(new_children);

    Ok(())
}

/// Reduces an arithmetic operation node by evaluating it if all children are constants.
///
/// If the node represents an arithmetic operation (`+`, `-`, `*`, `/`) and all its children
/// are constants (`Number` nodes), the operation is computed and the node is replaced with
/// a single `Number` node containing the result. If any child is non-constant, no simplification
/// is performed.
///
/// # Parameters
///
/// * `node_id` - The ID of the node to simplify.
/// * `expr` - The expression tree containing the node.
///
/// # Returns
///
/// * `Ok(())` if the simplification succeeds or is not applicable.
/// * `Err(ExprError)` if an arithmetic evaluation fails (e.g., division by zero).
fn reduce(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), LogicError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::Operation {
        return Ok(());
    }

    let op = match node.content().as_arithmetic_op() {
        Some(op) => op,
        None => return Ok(()),
    };

    let children = node.children().to_vec();
    let mut constant_values = Vec::with_capacity(children.len());
    let mut non_constant_children = Vec::with_capacity(children.len());

    for &child_id in &children {
        let child = expr.try_node(child_id)?;

        if let Some(val) = child.content().as_float() {
            // --- Logic: Absorbing Elements ---
            // x * 0 = 0
            if op == ArithmeticOp::Mul && val.0 == 0.0 {
                return replace_with_constant(node_id, expr, OrderedFloat(0.0));
            }

            // --- Logic: Neutral Elements ---
            // We skip them to simplify the node (e.g., x + 0 -> x)
            if is_neutral(val, op) {
                continue;
            }

            constant_values.push(val);
        } else {
            non_constant_children.push(child_id);
        }
    }

    // --- Decision Time ---
    if non_constant_children.is_empty() {
        // ... (ton code actuel pour le cas full constant est parfait) ...
        if constant_values.is_empty() {
            return replace_with_constant(node_id, expr, identity_element(op));
        }
        let result = evaluate_arithmetic_expression(op, &constant_values)?;
        replace_with_constant(node_id, expr, result)
    } else if !constant_values.is_empty() && (op == ArithmeticOp::Add || op == ArithmeticOp::Mul) {
        // On ne fait de réduction PARTIELLE que pour + et *
        apply_partial_reduction(node_id, expr, op, constant_values, non_constant_children)
    } else {
        // Pour - et /, si ce n'est pas 100% constant, on ne touche à rien
        // pour préserver l'ordre des opérations.
        Ok(())
    }
}

/// Returns true if the value is neutral for the given operator.
fn is_neutral(val: OrderedFloat<f64>, op: ArithmeticOp) -> bool {
    match op {
        ArithmeticOp::Add | ArithmeticOp::Sub => val.0 == 0.0,
        ArithmeticOp::Mul | ArithmeticOp::Div => val.0 == 1.0,
    }
}

/// Returns the identity (neutral) element for an operator.
fn identity_element(op: ArithmeticOp) -> OrderedFloat<f64> {
    match op {
        ArithmeticOp::Add | ArithmeticOp::Sub => OrderedFloat(0.0),
        ArithmeticOp::Mul | ArithmeticOp::Div => OrderedFloat(1.0),
    }
}

/// Utility to replace a node with a single constant Number.
fn replace_with_constant(node_id: NodeId, expr: &mut Expr, val: OrderedFloat<f64>) -> Result<(), LogicError> {
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(ExprKind::Number);
    node_mut.set_content(Content::Float(val));
    node_mut.set_children(vec![]);
    Ok(())
}

fn apply_partial_reduction(
    node_id: NodeId,
    expr: &mut Expr,
    op: ArithmeticOp,
    constants: Vec<OrderedFloat<f64>>,
    mut non_constants: Vec<NodeId>,
) -> Result<(), LogicError> {
    // 1. Evaluate the constant part (e.g., 10 + 20 -> 30)
    let final_constant = evaluate_arithmetic_expression(op, &constants)?;

    // 2. Add the constant to the children only if it's not neutral (e.g., skip +0 or *1)
    if !is_neutral(final_constant, op) {
        // Direct allocation of the Number node
        let const_node = ExprNode::new(
            ExprKind::Number,
            Content::Float(final_constant),
            None,
        );
        let const_node_id = expr.alloc(const_node);
        non_constants.push(const_node_id);
    }

    // 3. Finalize the parent node based on the remaining children
    match non_constants.len() {
        0 => {
            // All children were neutral: reduce the parent to the identity element (e.g., 0 for +)
            replace_with_constant(node_id, expr, identity_element(op))?;
        }
        1 => {
            // Optimization: (+ x) -> x.
            // We "lift" the single child to the parent's position using move_to.
            let single_child_id = non_constants[0];
            expr.move_to(single_child_id, node_id)?;
        }
        _ => {
            // Standard reduction: Update the parent's children with the new mix
            let node_mut = expr.try_node_mut(node_id)?;
            node_mut.set_children(non_constants);
        }
    }

    Ok(())
}
/// Evaluates an arithmetic expression on a slice of constant operands.
///
/// This function performs arithmetic evaluation for the four standard operators:
/// - `Add` (`+`)
/// - `Mul` (`*`)
/// - `Sub` (`-`)
/// - `Div` (`/`)
///
/// # Parameters
///
/// * `op` - The arithmetic operator to apply.
/// * `values` - A slice of `OrderedFloat<f64>` representing the operands. Each value is treated
///              as a constant operand in the computation.
///
/// # Returns
///
/// Returns a `Result`:
/// - `Ok(OrderedFloat<f64>)` containing the computed result if the evaluation succeeds.
/// - `Err(ExprError)` if the operation cannot be evaluated. This can happen in the following cases:
///     - The slice is empty (no operands to evaluate).
///     - Division by zero is attempted.
///   The returned `ExprError::ArithmeticEvaluationError` contains the operator and operands
///   that caused the failure, allowing detailed error reporting.
///
/// # Behavior
///
/// 1. **Addition (`Add`)**: Returns the sum of all operands. An empty slice results in an error.
/// 2. **Multiplication (`Mul`)**: Returns the product of all operands. An empty slice results in an error.
/// 3. **Subtraction (`Sub`)**:
///     - Single operand: returns the negation of the operand (`-x`).
///     - Multiple operands: left-associative subtraction (`x1 - x2 - x3 ...`).
/// 4. **Division (`Div`)**:
///     - Single operand: returns the reciprocal (`1 / x`).
///     - Multiple operands: left-associative division (`x1 / x2 / x3 ...`).
///     - Division by zero triggers an `ArithmeticEvaluationError`.
///
/// # Examples
///
/// ```rust
/// use ordered_float::OrderedFloat;
/// use aiplan4rust::lang::ArithmeticOp;
/// use aiplan4rust::lir::expr::ExprError;
///
/// let operands = &[OrderedFloat(4.0), OrderedFloat(2.0)];
/// let result = evaluate_arithmetic_op(ArithmeticOp::Div, operands).unwrap();
/// assert_eq!(result, OrderedFloat(2.0));
///
/// let empty: &[OrderedFloat<f64>] = &[];
/// assert!(evaluate_arithmetic_op(ArithmeticOp::Add, empty).is_err());
/// ```
///
/// # Notes
///
/// - This function is intended for **constant evaluation** during expression simplification.
/// - Operands are passed as a slice of `OrderedFloat<f64>` to maintain total ordering and avoid
///   floating-point equality pitfalls.
/// - Errors include the operator and operands to help with debugging and reporting in expression trees.
fn evaluate_arithmetic_expression(
    op: ArithmeticOp,
    values: &[OrderedFloat<f64>],
) -> Result<OrderedFloat<f64>, LogicError> {
    let (first, rest) = values.split_first()
        .ok_or_else(|| LogicError::arithmetic_evaluation_error(op, values.to_vec()))?;

    let result = match op {
        ArithmeticOp::Add => values.iter().copied().sum(),
        ArithmeticOp::Mul => values.iter().copied().product(),
        ArithmeticOp::Sub => rest.iter().copied().fold(*first, |acc, x| acc - x),
        ArithmeticOp::Div => rest.iter().try_fold(*first, |acc, x| {
            if x.0 == 0.0 {
                Err(LogicError::arithmetic_evaluation_error(op, values.to_vec()))
            } else {
                Ok(acc / *x)
            }
        })?,
    };

    Ok(result)
}

#[cfg(test)]
mod simplify_arithmetic_operation_tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use ordered_float::OrderedFloat;

    /// Input: (+ 2 3)
    /// Expected output: 5
    #[test]
    fn test_addition_of_constants() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (+ 2.0 3.0)
        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);
        let root = builder.add(vec![n2, n3]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Constant folding
        // This should identify the ADD node with numeric children and compute the sum
        reduce(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root should no longer be an ADD node, but a single Number node
        assert_eq!(expr.kind(), Some(ExprKind::Number));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));

        Ok(())
    }

    /// Input: (- 10 3 2)
    /// Expected output: 5
    #[test]
    fn test_subtraction_of_constants() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (- 10.0 3.0 2.0)
        let n10 = builder.number(10.0);
        let n3 = builder.number(3.0);
        let n2 = builder.number(2.0);
        // Passing a vector implies: n10 - n3 - n2
        let root = builder.sub(vec![n10, n3, n2]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Folding the subtraction
        reduce(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Number));

        let root_node = expr.try_root_node()?;
        // 10.0 - 3.0 - 2.0 = 5.0
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));

        Ok(())
    }

    /// Input: (* 2 3 4)
    /// Expected output: 24
    #[test]
    fn test_multiplication_of_constants() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (* 2.0 3.0 4.0)
        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);
        let n4 = builder.number(4.0);
        let root = builder.mul(vec![n2, n3, n4]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Folding the product
        reduce(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Number));

        let root_node = expr.try_root_node()?;
        // Verification: 2.0 * 3.0 * 4.0 = 24.0
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(24.0)));

        Ok(())
    }

    /// Input: (/ 20 2 2)
    /// Expected output: 5
    #[test]
    fn test_division_of_constants() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (/ 20.0 2.0 2.0)
        let n20 = builder.number(20.0);
        let n2a = builder.number(2.0);
        let n2b = builder.number(2.0);
        let root = builder.div(vec![n20, n2a, n2b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Constant folding for division
        reduce(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Number));

        let root_node = expr.try_root_node()?;
        // Calculation: (20 / 2) / 2 = 5.0
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));

        Ok(())
    }

    /// Input: (/ 5 0)
    /// Expected output: ArithmeticEvaluationError
    #[test]
    fn test_division_by_zero() {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (/ 5.0 0.0)
        let n5 = builder.number(5.0);
        let n0 = builder.number(0.0);
        let div = builder.div(vec![n5, n0]);

        builder.set_root(div).unwrap();
        let mut expr = builder.finish();

        // 2. Transformation: Attempt to fold the division
        let result = reduce(expr.root_id().unwrap(), &mut expr);

        // 3. Validation: Check for the specific error variant
        assert!(result.is_err(), "Folding division by zero should fail");

        match result {
            Err(LogicError::ArithmeticEvaluationError { op, values }) => {
                assert_eq!(op, ArithmeticOp::Div);
                assert_eq!(values, vec![OrderedFloat(5.0), OrderedFloat(0.0)]);
            }
            _ => panic!("Expected ArithmeticEvaluationError, got {:?}", result),
        }
    }
}
