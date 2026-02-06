use ordered_float::OrderedFloat;
use crate::aiplan4rust::lang::ArithmeticOp;
use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::tree::{NodeId, SyntaxContent};

/// Simplifies an arithmetic expression node.
///
/// This function centralizes the arithmetic simplification process, ensuring that
/// arithmetic expressions are normalized and constants are evaluated. It performs
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
///   the tree structure while simplifying expressions.
pub(super) fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
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

/// Flattens nested arithmetic expressions of the same operator.
///
/// This function normalizes arithmetic expressions by pulling up children from
/// nested operations of the same kind (`+` or `*`). It ensures that expressions
/// like nested sums or products are represented in a flat, normalized form.
///
/// # Examples
///
/// - `(+ 1 (+ 2 3) 4)` → `(+ 1 2 3 4)`
/// - `(* 2 (* 3 4))`   → `(* 2 3 4)`
///
/// # Parameters
///
/// * `node_id` - The ID of the arithmetic operation node to flatten.
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
) -> Result<(), ExprError> {
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
) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;

    // Only arithmetic operation nodes can be simplified
    if node.kind() != ExprKind::Operation {
        return Ok(());
    }

    // Extract the arithmetic operator
    let op = match node.content().as_arithmetic_op() {
        Some(op) => op,
        None => {
            debug_assert!(false, "Operation node without ArithmeticOp");
            return Ok(());
        }
    };

    // Collect constant (Number) children
    let mut values: Vec<OrderedFloat<f64>> = Vec::with_capacity(node.children().len());
    for &child_id in node.children() {
        let child = expr.try_node(child_id)?;
        if let Some(f) = child.content().as_float() {
            values.push(f);
        } else {
            // Cannot simplify if any child is non-constant
            return Ok(());
        }
    }

    if values.is_empty() {
        debug_assert!(false, "Arithmetic operation with no children");
        return Ok(());
    }

    // Evaluate the operation
    let result = evaluate_arithmetic_expression(op, &values)?;

    // Replace the node with a constant Number node
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(ExprKind::Number);
    node_mut.set_content(Content::Float(result));
    node_mut.set_children(vec![]);

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
) -> Result<OrderedFloat<f64>, ExprError> {
    let (first, rest) = values.split_first()
        .ok_or_else(|| ExprError::arithmetic_evaluation_error(op, values.to_vec()))?;

    let result = match op {
        ArithmeticOp::Add => values.iter().copied().sum(),
        ArithmeticOp::Mul => values.iter().copied().product(),
        ArithmeticOp::Sub => rest.iter().copied().fold(*first, |acc, x| acc - x),
        ArithmeticOp::Div => rest.iter().try_fold(*first, |acc, x| {
            if x.0 == 0.0 {
                Err(ExprError::arithmetic_evaluation_error(op, values.to_vec()))
            } else {
                Ok(acc / *x)
            }
        })?,
    };

    Ok(result)
}

/*#[cfg(test)]
mod simplify_arithmetic_operation_tests {
    use super::*;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
    use ordered_float::OrderedFloat;

    /// Input: (+ 2 3)
    /// Expected output: 5
    #[test]
    fn test_addition_of_constants() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);
        let add = builder.add(vec![n2, n3]);

        builder.set_root(add).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        reduce(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Number);
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));
        assert_eq!(output, "5");
    }

    /// Input: (- 10 3 2)
    /// Expected output: 5
    #[test]
    fn test_subtraction_of_constants() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let n10 = builder.number(10.0);
        let n3 = builder.number(3.0);
        let n2 = builder.number(2.0);
        let sub = builder.sub(vec![n10, n3, n2]);

        builder.set_root(sub).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        reduce(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Number);
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));
        assert_eq!(output, "5");
    }

    /// Input: (* 2 3 4)
    /// Expected output: 24
    #[test]
    fn test_multiplication_of_constants() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let n2 = builder.number(2.0);
        let n3 = builder.number(3.0);
        let n4 = builder.number(4.0);
        let mul = builder.mul(vec![n2, n3, n4]);

        builder.set_root(mul).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        reduce(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Number);
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(24.0)));
        assert_eq!(output, "24");
    }

    /// Input: (/ 20 2 2)
    /// Expected output: 5
    #[test]
    fn test_division_of_constants() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let n20 = builder.number(20.0);
        let n2a = builder.number(2.0);
        let n2b = builder.number(2.0);
        let div = builder.div(vec![n20, n2a, n2b]);

        builder.set_root(div).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        reduce(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Number);
        assert_eq!(root_node.content().as_float(), Some(OrderedFloat(5.0)));
        assert_eq!(output, "5");
    }

    /// Input: (/ 5 0)
    /// Expected output: ArithmeticEvaluationError
    #[test]
    fn test_division_by_zero() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let n5 = builder.number(5.0);
        let n0 = builder.number(0.0);
        let div = builder.div(vec![n5, n0]);

        builder.set_root(div).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);

        let result = reduce(expr.root_id().unwrap(), &mut expr);

        print!("{} -> error: {:?} ", input, result);

        assert!(result.is_err());
        if let Err(ExprError::ArithmeticEvaluationError { op, values }) = result {
            assert_eq!(op, ArithmeticOp::Div);
            assert_eq!(values, vec![OrderedFloat(5.0), OrderedFloat(0.0)]);
        } else {
            panic!("Expected ArithmeticEvaluationError");
        }
    }
}*/
