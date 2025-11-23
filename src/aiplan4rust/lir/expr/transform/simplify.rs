use std::collections::HashSet;
use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};

/// Simplifies an expression tree.
///
/// This function performs the following simplifications on a logical expression:
///
/// 1. **Flattening nested AND/OR nodes**:
///    If an AND node contains child nodes that are themselves ANDs (or OR nodes containing ORs),
///    their children are merged directly into the parent node.
///    Example: `(and (and A B) C)` → `(and A B C)`
///
/// 2. **Removing duplicate children**:
///    Duplicate child nodes of an AND or OR are removed.
///    Example: `(and A A B)` → `(and A B)`
///
/// 3. **Reducing single-child logical nodes**:
///    If an AND or OR node ends up with only one child, it is replaced by that child.
///    Example: `(and A)` → `A`
///
/// # Arguments
/// * `expr` - The expression to simplify. Must be a valid `Expr` tree.
///
/// # Returns
/// * `Ok(())` if simplification succeeds.
/// * `Err(ExprError)` if any internal error occurs (e.g., invalid node ID).
///
/// # Example
/// ```rust
/// let mut expr = Expr::parse("(and (and A B) C)").unwrap();
/// simplify(&mut expr).unwrap();
/// assert_eq!(expr.to_string(), "(and A B C)");
/// ```
pub fn simplify(expr: &mut Expr) -> Result<(), ExprError> {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return Ok(()), // tree empty, nothing to do
    };

    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;
        let node_kind = node.kind();

        if node_kind == ExprKind::And || node_kind == ExprKind::Or {
            let mut tmp_children = Vec::new();
            let mut seen_ids = HashSet::new();

            // Instead of cloning, we take ownership of the children
            let children_ids = std::mem::take(expr.try_node_mut(node_id)?.children_mut());

            for child_id in children_ids {
                let child_node = expr.try_node(child_id)?;
                if child_node.kind() == node_kind {
                    // Flatten grandchildren directly
                    let grand_children = std::mem::take(expr.try_node_mut(child_id)?.children_mut());
                    for grandchild_id in grand_children {
                        if seen_ids.insert(grandchild_id) {
                            tmp_children.push(grandchild_id);
                        }
                    }
                } else if seen_ids.insert(child_id) {
                    tmp_children.push(child_id);
                }
            }

            // Replace children with flattened result
            expr.try_node_mut(node_id)?.set_children(tmp_children);
        }

        // Push children for postorder processing
        stack.extend(expr.try_node(node_id)?.children());
    }

    // Adjust root if it has a single child after simplification
    if let Some(root_id) = expr.root_id() {
        let root_node = expr.try_node(root_id)?;
        if matches!(root_node.kind(), ExprKind::And | ExprKind::Or)
            && root_node.children().len() == 1
        {
            expr.set_root_id(root_node.children()[0])?;
        }
    }

    Ok(())
}


/// Élimine les constantes `true` / `false` dans les AND/OR.
/// Exemple: `(and A true) -> A`, `(or A false) -> A`.
pub fn eliminate_constants(expr: &mut Expr) {
    // TODO: implémenter la simplification des constantes
}

/// Nettoyage final après toutes les transformations
/// Supprime les nœuds vides et triviales restantes.
pub fn simplify_final(expr: &mut Expr) {
    // TODO: implémenter nettoyage final
}


#[cfg(test)]
mod tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
    use crate::aiplan4rust::lir::expr::transform::simplify::simplify;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test that nested AND nodes are correctly flattened.
    ///
    /// Input: (and (and A B) C)
    /// Expected: (and A B C)
    #[test]
    fn test_flatten_nested_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        // Create predicates A, B, C
        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");

        // Build nested AND: (and A B)
        let inner_and = builder.and(vec![a, b]);
        // Build root AND: (and (and A B) C)
        let root = builder.and(vec![inner_and, c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        // Save input for printing
        let input = expr.to_syntax_string(&interner);
        // Simplify expression
        simplify(&mut expr).unwrap();
        // Save result for printing
        let result = expr.to_syntax_string(&interner);

        // Print single-line summary: input -> result
        println!("\n{} -> {}", input, result);

        // Verify root now has 3 children: A, B, C
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.children().len(), 3);
        // Verify all children are predicates
        assert!(root_node.children().iter().all(|&id| {
            expr.try_node(id).unwrap().kind() == ExprKind::Predicate
        }));
    }

    /// Test that nested OR nodes are correctly flattened.
    ///
    /// Input: (or (or A B) C)
    /// Expected: (or A B C)
    #[test]
    fn test_flatten_nested_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");

        // Build nested OR
        let inner_or = builder.or(vec![a, b]);
        let root = builder.or(vec![inner_or, c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let result = expr.to_syntax_string(&interner);

        println!("\n{} -> {}", input, result);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.children().len(), 3);
        assert!(root_node.children().iter().all(|&id| {
            expr.try_node(id).unwrap().kind() == ExprKind::Predicate
        }));
    }

    /// Test that AND/OR nodes with a single child are replaced by the child itself.
    ///
    /// Input: (and A)
    /// Expected: A
    #[test]
    fn test_single_child_and_or_replacement() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        // Single-child AND node
        let a = builder.predicate("A");
        let inner_and = builder.and(vec![a]);
        builder.set_root(inner_and).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let result = expr.to_syntax_string(&interner);

        println!("\n{} -> {}", input, result);

        // After simplification, root should be the predicate itself
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Predicate);
    }

    /// Test a more complex tree with mixed AND/OR nodes.
    ///
    /// Input: (and (and A (or B C)) D)
    /// Expected: (and A (or B C) D)
    #[test]
    fn test_nested_mixed_and_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");
        let d = builder.predicate("D");

        // Nested OR inside AND
        let inner_or = builder.or(vec![b, c]);
        let inner_and = builder.and(vec![a, inner_or]);
        let root = builder.and(vec![inner_and, d]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let result = expr.to_syntax_string(&interner);

        println!("\n{} -> {}", input, result);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);
        let kinds: Vec<_> = root_node.children().iter()
            .map(|&id| expr.try_node(id).unwrap().kind())
            .collect();
        assert_eq!(kinds, vec![ExprKind::Predicate, ExprKind::Or, ExprKind::Predicate]);
    }

    /// Test simplification on an empty expression tree.
    ///
    /// Input: (empty)
    /// Expected: None (tree remains empty)
    #[test]
    /// Test simplification on an empty expression tree.
    ///
    /// Input: (empty)
    /// Expected: None (tree remains empty)
    #[test]
    fn test_empty_tree() {
        let mut interner = StringInterner::new();
        let mut expr = Expr::new();

        // Convert input expression to string for debugging
        let input = if expr.root_id().is_none() {
            "(empty)".to_string()
        } else {
            expr.to_syntax_string(&interner)
        };

        // Simplify the expression
        simplify(&mut expr).unwrap();

        // Convert result to string for printing
        let result = if expr.root_id().is_none() {
            "(empty)".to_string()
        } else {
            expr.to_syntax_string(&interner)
        };

        // Print single-line summary
        println!("\n{} -> {}", input, result);

        // After simplification, tree should still be empty
        assert!(expr.root_id().is_none());
    }

    /// Test that duplicate children are removed during simplification.
    ///
    /// Input: (and A A B)
    /// Expected: (and A B)
    #[test]
    fn test_deduplicate_children() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        // Duplicate child nodes
        let root = builder.and(vec![a, a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        // Convert input expression to string for debugging
        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let result = expr.to_syntax_string(&interner);

        println!("\n{} -> {}", input, result);

        // After simplification, the root should still be 'And'
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);

        // Children should be deduplicated
        let children = root_node.children();
        assert_eq!(children.len(), 2);
        assert!(children.contains(&a));
        assert!(children.contains(&b));
    }

}
