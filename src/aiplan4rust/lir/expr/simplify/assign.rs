use ordered_float::OrderedFloat;
use crate::aiplan4rust::lang::AssignOp;
use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, NodeId, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::SyntaxContent;

/// Represents the numeric value 0.0, used for detecting trivial `increase` or `decrease` assignments.
const ZERO: OrderedFloat<f64> = OrderedFloat(0.0);

/// Represents the numeric value 1.0, used for detecting trivial `scale-up` or `scale-down` assignments.
const ONE: OrderedFloat<f64> = OrderedFloat(1.0);

/// Simplifies an `Assign` node in the expression tree by simplifying trivial assignments.
///
/// This function inspects the assignment operator and its value, and transforms
/// trivial assignments (e.g., `increase 0`, `decrease 0`, `scale-up 1`, `scale-down 1`)
/// into a trivially true `And` node. Regular `assign` operations and non-trivial
/// values are left unchanged.
///
/// # Parameters
/// - `node_id`: The ID of the node to normalize. Must correspond to an `Assign` node.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Err(ExprError)` if accessing nodes or contents fails.
///
/// # Notes
/// - This function is intended for internal use within the parent module,
///   and should be called as part of the assignment normalization pipeline.
/// - The function preserves the expression tree structure by replacing trivial
///   assignments rather than removing nodes from the parent.
pub(super) fn simplify(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    simplify_trivial_assignments(node_id, expr)?;
    Ok(())
}

/// Simplifies trivial assignment nodes in the expression tree.
///
/// Transforms assignments like `(increase <f> 0)` or `(scale-up <f> 1)`
/// into a trivial `(and)` node (always true), leaving other assignments untouched.
///
/// # Parameters
/// - `node_id`: ID of the node to simplify (must be of kind `Assign`).
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if the node was simplified.
/// - `Ok(false)` if no simplification was done.
/// - `Err(ExprError)` if node access fails.
///
/// # Notes
/// - The function uses `debug_assert!` to enforce internal invariants:
///     - The node must be of kind `Assign`.
///     - The assign node must have exactly 2 children: the target and the value.
///     - The first child (target) must be of kind `FunctionTerm`.
/// - These assertions are only active in debug builds and are intended to catch
///   programming errors during development. They do not affect release behavior.
pub fn simplify_trivial_assignments(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Assign, "Node must be an Assign");

    let assign_op = node.content().try_assign_op()?;
    let children = node.children();
    debug_assert!(children.len() == 2, "Assign node must have exactly 2 children");

    let target_id = children[0];
    let value_id = children[1];

    let target = expr.try_node(target_id)?;
    debug_assert!(
        matches!(target.kind(), ExprKind::FunctionTerm),
        "First child of Assign must be a function term"
    );

    let value = expr.try_node(value_id)?;
    if is_trivial_assign_value(value, assign_op)? {
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(ExprKind::And);
        node_mut.set_content(Content::None);
        node_mut.set_children(vec![]);
        return Ok(true);
    }

    Ok(false)
}

/// Returns true if the expression node represents a trivial value
/// for the given assignment operator.
///
/// Examples of trivial assignments:
/// - `(increase <f> 0)`
/// - `(decrease <f> 0)`
/// - `(scale-up <f> 1)`
/// - `(scale-down <f> 1)`
fn is_trivial_assign_value(
    node: &ExprNode,
    assign_op: AssignOp,
) -> Result<bool, ExprError> {
    if node.kind() != ExprKind::Number {
        return Ok(false);
    }
    let number = node.content().try_float()?;

    match assign_op {
        AssignOp::Increase | AssignOp::Decrease => Ok(number == ZERO),
        AssignOp::ScaleUp | AssignOp::ScaleDown => Ok(number == ONE),
        AssignOp::Assign => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::interner::StringInterner;
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test trivial (increase (F) 0) -> (and)
    #[test]
    fn test_trivial_increase() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let value = builder.number(0.0);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.increase(target, value);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::And);
        assert!(node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test trivial (decrease (F) 0) -> (and)
    #[test]
    fn test_trivial_decrease() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let value = builder.number(0.0);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.decrease(target, value);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::And);
        assert!(node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test trivial (scale-up (F) 1) -> (and)
    #[test]
    fn test_trivial_scale_up() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let value = builder.number(1.0);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.scale_up(target, value);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::And);
        assert!(node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test assign operator (Assign) is never simplified
    #[test]
    fn test_assign_op_assign() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let value = builder.number(0.0);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.assign(target, value);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::Assign);
        assert_eq!(node.children().len(), 2);
        assert_eq!(output, "(assign (F) 0)");
    }

    /// Test assign with value as a function -> not simplified
    #[test]
    fn test_assign_value_function() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let value = builder.function_term("V", vec![]);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.increase(target, value);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::Assign);
        assert_eq!(output, "(increase (F) (V))");
    }

    /// Test assign with value as an arithmetic operation -> not simplified
    #[test]
    fn test_assign_value_arithmetic() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let left = builder.number(2.0);
        let right = builder.number(3.0);
        let op = builder.add(vec![left, right]);
        let target = builder.function_term("F", vec![]);
        let assign_node = builder.increase(target, op);

        builder.set_root(assign_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(assign_node, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let node = expr.try_node(assign_node).unwrap();
        assert_eq!(node.kind(), ExprKind::Assign);
        assert_eq!(output, "(increase (F) (+ 2 3))")
    }

}
