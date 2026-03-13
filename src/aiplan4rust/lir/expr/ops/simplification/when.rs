use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::NodeId;

/// Simplifies a `When` expression node according to PDDL simplification rules.
///
/// This function delegates to [`simplify_when_node`] to reduce a `When` node
/// into a simpler, canonical form. Normalization may modify the expression
/// tree by removing redundant constructs or replacing the `When` node entirely.
///
/// # Parameters
/// - `node_id`: Identifier of the `When` node to normalize.
/// - `logic`: Mutable reference to the expression tree containing the node.
///
/// # Returns
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
/// normalize(when_id, &mut logic)?;
/// ```
pub fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprOpError> {
    simplify_when_node(node_id, expr)?;
    Ok(())
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
/// - `node_id`: The ID of the `When` node to simplification.
/// - `logic`: Mutable reference to the expression tree.
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
fn simplify_when_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::When, "Node must be a When");

    let children = node.children();
    debug_assert!(children.len() == 2, "When node must have exactly 2 children");

    let cond_id = children[0];
    let eff_id = children[1];

    // Case 1: (when (and) E) -> E
    if expr.is_empty_and(cond_id)? {
        expr.move_to(eff_id, node_id)?;
        return Ok(true);
    }

    // Case 2: (when (or) E) -> (and)
    if expr.is_empty_or(cond_id)? {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 3: (when C (and)) -> (and)
    if expr.is_empty_and(eff_id)? {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 4: (when E E) -> (and)
    if expr.deep_sub_expr_eq(cond_id, eff_id)? {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;

    /// Test case 1: (when (and) E) -> E
    #[test]
    fn test_when_empty_and_condition() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (when (and) (E))
        let atomic_e = builder.atomic_formula(1, vec![]); // "E"
        let empty_and = builder.and(vec![]);
        let when_node = builder.when(empty_and, atomic_e);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: (when True E) -> E
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le 'when' a disparu, il ne reste que l'AtomicFormula
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test case 2: (when (or) E) -> (and)
    #[test]
    fn test_when_empty_or_condition() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (when (or) (E)) -> condition toujours fausse
        let atomic_e = builder.atomic_formula(1, vec![]);
        let empty_or = builder.or(vec![]);
        let when_node = builder.when(empty_or, atomic_e);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: simplification (condition fausse -> l'effet disparaît)
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le 'when' est remplacé par un 'and' vide (effet neutre)
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test case 3: (when E E) -> (and)
    #[test]
    fn test_when_identical_condition_effect() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (when (E) (E))
        let atomic_e = builder.atomic_formula(1, vec![]);
        let when_node = builder.when(atomic_e, atomic_e);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: simplification
        // Un effet qui ne change pas l'état est inutile.
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le résultat doit être un effet neutre (and vide)
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test case 4: (when C (and)) -> (and)
    #[test]
    fn test_when_empty_effect() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (when (C) (and))
        let atomic_c = builder.atomic_formula(1, vec![]);
        let empty_and = builder.and(vec![]);
        let when_node = builder.when(atomic_c, empty_and);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: simplification
        // Si l'effet est vide, le 'when' est inutile.
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le 'when' devient un 'and' vide.
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test case 5: No simplification applied (when (C) (E)) -> (when (C) (E))
    #[test]
    fn test_when_no_simplification() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (when (C) (E))
        let atomic_c = builder.atomic_formula(1, vec![]); // Condition "C"
        let atomic_e = builder.atomic_formula(2, vec![]); // Effet "E"
        let when_node = builder.when(atomic_c, atomic_e);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: simplification (no change)
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le Kind doit rester ExprKind::When
        assert_eq!(root_node.kind(), ExprKind::When);

        // On vérifie que les deux enfants (condition et effet) sont toujours là
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        let cond_node = expr.try_node(children[0])?;
        let eff_node = expr.try_node(children[1])?;
        assert_eq!(cond_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(eff_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }
}
