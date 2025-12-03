use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::syntax::tree::NodeId;

/// Normalizes a `When` expression node according to PDDL simplification rules.
///
/// This function delegates to [`simplify_when_node`] to reduce a `When` node
/// into a simpler, canonical form. Normalization may modify the expression
/// tree by removing redundant constructs or replacing the `When` node entirely.
///
/// # Parameters
/// - `node_id`: Identifier of the `When` node to normalize.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(true)` if the node was simplified.
/// - `Ok(false)` if no simplification was applicable.
/// - `Err(ExprError)` if accessing or modifying the node fails.
///
/// # Errors
/// Returns an error if `node_id` does not correspond to a valid node
/// in the expression tree or if rewriting fails.
///
/// # Examples
/// ```rust
/// // Given: (when (and) E)
/// // Result: E
/// normalize(when_id, &mut expr)?;
/// ```
pub(super) fn normalize(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    simplify_when_node(node_id, expr)
}

/// Simplifies a `When` node according to PDDL logical rewriting rules.
///
/// This function applies the following simplification rules:
///
/// 1. **Empty conjunction as condition:**
///    `(when (and) E)` → `E`
///
/// 2. **Empty disjunction as condition:**
///    `(when (or) E)` → `(and)`
///    (An effect guarded by an empty disjunction simplifies to an empty conjunction.)
///
/// 3. **Identical condition and effect:**
///    `(when E E)` → `(and)`
///
/// 4. **Empty conjunction as effect:**
///    `(when C (and))` → `(and)`
///
/// After applying any of these rules, the original `When` node is replaced or
/// rewritten in-place.
///
/// # Parameters
/// - `node_id`: The ID of the `When` node to simplify.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if a simplification was performed.
/// - `Ok(false)` if no rule matched.
/// - `Err(ExprError)` if the node cannot be accessed or rewritten.
///
/// # Panics
/// Panics in debug mode if `node_id` does not refer to a `When` node.
///
/// # Notes
/// This function should only be called on nodes whose kind is
/// `ExprKind::When`. It directly modifies the expression tree to apply
/// simplifications.
fn simplify_when_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::When, "Node must be a When");

    let children = node.children();
    debug_assert!(children.len() == 2, "When node must have exactly 2 children");

    let cond_id = children[0];
    let eff_id = children[1];

    let cond = expr.try_node(cond_id)?;
    let eff = expr.try_node(eff_id)?;

    // Case 1: (when (and) E) -> E
    if cond.kind() == ExprKind::And {
        expr.move_to(eff_id, node_id)?;
        return Ok(true);
    }

    // Case 2: (when (or) E) -> (and)
    if cond.kind() == ExprKind::Or {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 3: (when E E) -> (and)
    if cond_id == eff_id {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 4: (when C (and)) -> (and)
    if eff.kind() == ExprKind::And {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test case 1: (when (and) E) -> E
    #[test]
    fn test_when_empty_and_condition() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_e = builder.atomic_formula("E", vec![]);
        let empty_and = builder.and(vec![]);
        let when_node = builder.when(empty_and, atomic_e);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
    }

    /// Test case 2: (when (or) E) -> (and)
    #[test]
    fn test_when_empty_or_condition() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_e = builder.atomic_formula("E", vec![]);
        let empty_or = builder.or(vec![]);
        let when_node = builder.when(empty_or, atomic_e);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
    }

    /// Test case 3: (when E E) -> (and)
    #[test]
    fn test_when_identical_condition_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_e = builder.atomic_formula("E", vec![]);
        let when_node = builder.when(atomic_e, atomic_e);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
    }

    /// Test case 4: (when C (and)) -> (and)
    #[test]
    fn test_when_empty_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_c = builder.atomic_formula("C", vec![]);
        let empty_and = builder.and(vec![]);
        let when_node = builder.when(atomic_c, empty_and);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
    }

    /// Test case 5: No simplification applied (when (C) (E)) -> (when (C) (E))
    #[test]
    fn test_when_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_c = builder.atomic_formula("C", vec![]);
        let atomic_e = builder.atomic_formula("E", vec![]);
        let when_node = builder.when(atomic_c, atomic_e);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        let changed = normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert!(!changed);
        assert_eq!(root_node.kind(), ExprKind::When);
        let children = root_node.children();
        assert_eq!(children.len(), 2, "When node should have exactly 2 children");
        let cond_node = expr.try_node(children[0]).unwrap();
        let eff_node = expr.try_node(children[1]).unwrap();
        assert_eq!(cond_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(eff_node.kind(), ExprKind::AtomicFormula);

    }
}
