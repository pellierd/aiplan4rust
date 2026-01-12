use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Recursively pushes negations down the expression tree using De Morgan’s laws
/// and quantifier negation rules.
///
/// This function ensures that negations (`Not` nodes) are propagated down to atomic
/// formulas, preparing the expression tree for subsequent processing such as temporal
/// factorization or simplification.
///
/// # Transformations applied
/// 1. **De Morgan’s laws**:
///    - `Not(And(...))` → `Or(Not(...))`
///    - `Not(Or(...))` → `And(Not(...))`
/// 2. **Quantifier negation rules**:
///    - `Not(Forall x φ)` → `Exists x Not(φ)`
///    - `Not(Exists x φ)` → `Forall x Not(φ)`
///
/// # Preconditions
/// - Typically called after implications have been eliminated via `eliminate_imply`.
/// - This function is part of the **normalization pipeline**, orchestrated by the
///   `normalize` module. Users should not call this directly unless implementing
///   a custom normalization sequence.
///
/// # Parameters
/// - `root_id`: NodeId of the root of the subtree to process.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(())` if all negations are successfully pushed down.
/// - `Err(ExprError::invalid_expr_node)` if a `Not` node has a child that is **invalid**
///   for negation propagation. Only the following kinds are supported as children:
///   `Not`, `And`, `Or`, `Forall`, `Exists`, `FComp`, or atomic formulas. Any other kind
///   triggers this error.
/// - `Err(ExprError)` if accessing or modifying nodes fails.
///
/// # Notes
/// - Mutates the tree in place.
/// - Uses a stack for depth-first traversal to handle `Not` nodes.
/// - Newly created `Not` nodes are pushed onto the stack for further processing.
/// - Assumes that each `Not` node has exactly one child; verified with a `debug_assert!`.
/// - After this step, all literals are in a form suitable for temporal specifier propagation
///   (`push_time_specifier`) and factorization (`factorize_time_specifier`).
///
/// # Example usage
/// ```rust
/// // Part of the normalization pipeline managed by the `normalize` module
/// push_negation(root_id, &mut expr)?;
/// ```
pub fn push_negation(root_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;
        if node.kind() != ExprKind::Not {
            continue;
        }

        let children = node.children();
        debug_assert!(
            children.len() == 1,
            "Not node must have exactly one child, found {}",
            children.len()
        );

        // Retrieve the single child of the Not node
        let child_id = children[0];
        let child = expr.try_node(child_id)?;

        match child.kind() {
            ExprKind::And | ExprKind::Or => {
                // Apply De Morgan's law and push new Not nodes onto the stack
                let new_not_ids = apply_de_morgan(node_id, expr)?;
                stack.extend(new_not_ids);
            }
            ExprKind::Forall | ExprKind::Exists => {
                // Apply quantifier negation rules and push the new Not node onto the stack
                let new_not_id = apply_quantifier_negation(node_id, expr)?;
                stack.push(new_not_id);
            }
            ExprKind::Not | ExprKind::FComp | ExprKind::AtomicFormula => {
                continue;
            }
            _ => {
                return Err(ExprError::invalid_expr_node(child_id, child.kind()));
            }
        }
    }

    Ok(())
}

/// Applies De Morgan’s law to a `Not` node whose child is an `And` or `Or`.
///
/// Specifically, this function transforms a `Not` applied to a conjunction or
/// disjunction by pushing the negation down to each operand:
/// - `(not (and A B ...))` → `(or (not A) (not B) ...)`
/// - `(not (or A B ...))` → `(and (not A) (not B) ...)`
///
/// This is part of the `push_negations` preprocessing, which moves negations
/// down the expression tree without simplifying double negations. Newly created
/// `Not` nodes are returned so they can be processed further.
///
/// # Parameters
/// - `node_id`: NodeId of the `Not` node to rewrite.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Vec<NodeId>` containing the newly created `Not` nodes for each child of
///   the original And/Or. These should be pushed onto the stack for further
///   processing by `push_negations`.
///
/// # Debug assertions
/// - The node must be a `Not`.
/// - The `Not` node must have exactly one child.
/// - The child of the `Not` node must be either an `And` or `Or`.
///
/// # Panics / Errors
/// - Returns `ExprError` if any node cannot be accessed or mutated.
#[allow(dead_code)]
fn apply_de_morgan(node_id: NodeId, expr: &mut Expr) -> Result<Vec<NodeId>, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(
        node.kind() == ExprKind::Not,
        "apply_de_morgan called on non-Not node"
    );

    let children = node.children();
    debug_assert!(
        children.len() == 1,
        "Not node must have exactly one child, found {}",
        children.len()
    );

    let child_id = children[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(
        child.kind() == ExprKind::And || child.kind() == ExprKind::Or,
        "Child of Not must be And or Or for apply_de_morgan"
    );

    // Clone the info we need before taking a mutable borrow
    let child_kind = child.kind();
    let grand_children: Vec<NodeId> = child.children().to_vec();

    // Mutate the parent Not node into Or/And
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(if child_kind == ExprKind::And { ExprKind::Or } else { ExprKind::And });
    node_mut.set_content(Content::None);
    node_mut.set_children(vec![]);

    // Create new Not nodes for each child of the original And/Or
    let mut new_not_ids = Vec::new();
    for &gc in &grand_children {
        let new_not = ExprNode::new(ExprKind::Not, Content::None, Some(node_id));
        let new_not_id = expr.alloc_with_children(new_not, vec![gc]);
        new_not_ids.push(new_not_id);
    }

    // Attach the new Not children to the parent node
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_children(new_not_ids.clone());

    Ok(new_not_ids)
}

/// Applies logical negation to a quantifier according to standard rules.
///
/// Specifically, this function transforms a `Not` node applied to a quantifier:
/// - `Not(Forall x φ)` → `Exists x Not(φ)`
/// - `Not(Exists x φ)` → `Forall x Not(φ)`
///
/// This is part of the `push_negations` preprocessing: it pushes negations down
/// the tree without simplifying double negations, allowing further processing
/// or normalization later.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the `Not` node in the expression tree.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `NodeId` of the newly created `Not` node applied to the quantifier body.
///   This node should be pushed onto the processing stack for further `push_negations`.
///
/// # Debug assertions
/// - The node must be a `Not`.
/// - The `Not` node must have exactly one child.
/// - The child must be a quantifier (`Forall` or `Exists`) with exactly two children:
///   a variable list and a body.
///
/// # Panics / Errors
/// - Returns `ExprError` if the tree cannot be accessed or mutated correctly.
#[allow(dead_code)]
fn apply_quantifier_negation(node_id: NodeId, expr: &mut Expr) -> Result<NodeId, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");

    let child_id = node.children()[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(
        child.kind() == ExprKind::Forall || child.kind() == ExprKind::Exists,
        "Child of Not must be a quantifier"
    );
    debug_assert!(child.children().len() == 1, "Quantifier node must have exactly one child (the body)");


    // Copy values to avoid borrow conflicts
    let child_kind = child.kind();
    let body_id = child.children()[0];

    //  Prendre les variables du quantificateur avant de muter node
    let child_mut = expr.try_node_mut(child_id)?;
    let quant_vars = std::mem::take(child_mut.content_mut());

    // Create a new Not node over the quantifier body
    let new_not = ExprNode::new(ExprKind::Not, Content::None, Some(node_id));
    let new_not_id = expr.alloc_with_children(new_not, vec![body_id]);

    // Mutate the original Not node into the opposite quantifier
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(match child_kind {
        ExprKind::Forall => ExprKind::Exists,
        ExprKind::Exists => ExprKind::Forall,
        _ => unreachable!("Child must be a quantifier"),
    });

    node_mut.set_content(quant_vars);
    node_mut.set_children(vec![new_not_id]);

    Ok(new_not_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

    /// Test pushing negation through AND using De Morgan's law.
    /// Input: (not (and (A) (B))) -> (or (not (A)) (not (B)))
    #[test]
    fn test_push_negation_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let and_node = builder.and(vec![a, b]);
        let root = builder.not(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);
        for &child_id in root_node.children() {
            let child = expr.try_node(child_id).unwrap();
            assert_eq!(child.kind(), ExprKind::Not);
        }

        assert_eq!(output, "(or (not (A)) (not (B)))");
    }

    /// Test pushing negation through OR using De Morgan's law.
    /// Input: (not (or (A) (B))) -> (and (not (A)) (not (B)))
    #[test]
    fn test_push_negation_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let or_node = builder.or(vec![a, b]);
        let root = builder.not(or_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        for &child_id in root_node.children() {
            let child = expr.try_node(child_id).unwrap();
            assert_eq!(child.kind(), ExprKind::Not);
        }

        assert_eq!(output, "(and (not (A)) (not (B)))");
    }

    /// Test pushing negation through a Forall quantifier.
    /// Input: (not (forall x (A))) -> (exists x (not (A)))
    #[test]
    fn test_push_negation_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let forall_node = builder.forall_with_string_vars(vec![("?X", "T")], a);
        let root = builder.not(forall_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        let children = root_node.children();
        assert_eq!(children.len(), 1);
        let body_node = expr.try_node(children[0]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(exists (?X - T) (not (A)))");
    }

    /// Test pushing negation through an Exists quantifier.
    /// Input: (not (exists x (A))) -> (forall x (not (A)))
    #[test]
    fn test_push_negation_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let exists_node = builder.exists_with_string_vars(vec![("?X", "T")], a);
        let root = builder.not(exists_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        let children = root_node.children();
        assert_eq!(children.len(), 1);
        let body_node = expr.try_node(children[0]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(forall (?X - T) (not (A)))");
    }

    /// Test that no transformation occurs for a NOT whose child is neither AND/OR nor quantifier.
    /// Input: (not (A)) -> unchanged
    #[test]
    fn test_push_negation_no_change() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let root = builder.not(a);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(output, "(not (A))");
    }

    /// Test pushing negation through a nested expression with AND, NOT, and quantifiers.
    /// Input: (not (and (A) (not (B)) (exists (?X) (C))))
    /// Expected output from push_negations alone:
    /// (or (not (A)) (not (not (B))) (forall (?X) (not (C))))
    #[test]
    fn test_push_negation_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let not_b = builder.not(b);
        let exists_c = builder.exists_with_string_vars(vec![("?X", "T")], c);
        let and_node = builder.and(vec![a, not_b, exists_c]);
        let root = builder.not(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 3);

        let first = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(first.kind(), ExprKind::Not);
        let second = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(second.kind(), ExprKind::Not);
        let second_child = expr.try_node(second.children()[0]).unwrap();
        assert_eq!(second_child.kind(), ExprKind::Not);
        let third = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(third.kind(), ExprKind::Forall);
        assert_eq!(third.children().len(), 1);
        let body_node = expr.try_node(third.children()[0]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(or (not (A)) (not (not (B))) (forall (?X - T) (not (C))))");
    }

    /// Test pushing negation through a deeply nested expression with AND, OR, NOT, and quantifiers.
    /// Input: (not (and (A) (not (or (B) (C))) (forall (?X) (exists (?Y) (D)))))
    /// Expected (after push_negations only, no simplification):
    /// (or (not (A)) (not (not (or (B) (C)))) (exists (?X) (forall (?Y) (not (D)))))
    #[test]
    fn test_push_negation_deep_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let d = builder.atomic_formula("D", vec![]);
        let or = builder.or(vec![b, c]);
        let not_or_bc = builder.not(or);

        let exists_d = builder.exists_with_string_vars(vec![("?Y", "T")], d);
        let forall_exists_d = builder.forall_with_string_vars(vec![("?X", "T")], exists_d);

        let and_node = builder.and(vec![a, not_or_bc, forall_exists_d]);
        let root = builder.not(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        push_negation(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 3);
        let first = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(first.kind(), ExprKind::Not);
        let second = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(second.kind(), ExprKind::Not);
        let third = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(third.kind(), ExprKind::Exists);
        assert_eq!(third.children().len(), 1);
        let body_node = expr.try_node(third.children()[0]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Forall);
        assert_eq!(body_node.children().len(), 1);
        let inner_body = expr.try_node(body_node.children()[0]).unwrap();
        assert_eq!(inner_body.kind(), ExprKind::Not);
        assert_eq!(
            output,
            "(or (not (A)) (not (not (or (B) (C)))) (exists (?X - T) (forall (?Y - T) (not (D)))))"
        );
    }
}
