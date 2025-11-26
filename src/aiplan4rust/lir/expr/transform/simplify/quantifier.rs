use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode};

/// Simplifies a quantified node (`forall` or `exists`) in a PDDL expression tree.
///
/// This function applies two simplifications to the specified quantifier node:
///
/// 1. **Remove empty quantifier**: if the quantifier has an empty variable list,
///    the node is replaced by its body.
/// 2. **Fuse nested quantifiers**: if the node has a child quantifier of the same type,
///    the variables from both quantifiers are combined, and the body of the inner
///    quantifier replaces the body of the outer node.
///
/// It assumes that all children of the node have already been simplified (post-order traversal).
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node to simplify.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if simplification completes successfully or no simplification is applicable.
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Panics
/// This function relies on `remove_empty_quantifier` and `fuse_nested_quantifiers`
/// to perform `debug_assert!` checks on AST invariants:
/// - Quantifier node must have at least two children (TypedList + body).
/// - TypedList nodes must be of kind `TypedList`.
/// - Nested quantifiers must also respect the two-children invariant.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// simplify_quantified_node(node_id, &mut expr)?;
/// ```
pub(in crate::aiplan4rust::lir::expr::transform::simplify) fn simplify_quantifier(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // 1. remove (forall() body)
    if remove_empty_quantifier(node_id, expr)? {
        return Ok(());
    }

    // 2. fuse nested quantifiers
    fuse_nested_quantifiers(node_id, expr)?;

    // 3. trivial body (forall (x) (and)) → (and)
    if simplify_quantifier_trivial_body(node_id, expr)? {
        return Ok(());
    }

    Ok(())
}

/// Simplifies a quantifier node by removing it if its variable list is empty.
///
/// This function handles nodes of kind `forall` or `exists`. If the first child,
/// which is the `TypedList` of quantified variables, is empty, the quantifier is
/// removed and the node is replaced by its body. The function assumes that the
/// children of the quantifier have already been simplified.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node to simplify.
/// - `expr`: A mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(true)` if the quantifier was removed and replaced by its body.
/// - `Ok(false)` if the quantifier was not removed (variable list is not empty
///   or AST is malformed).
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The quantifier node does not have at least two children (TypedList + body).
/// - The first child is not a `TypedList`.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// remove_empty_quantifier(node_id, &mut expr)?;
/// ```
#[allow(dead_code)]
fn remove_empty_quantifier(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    if node.kind() != ExprKind::Forall && node.kind() != ExprKind::Exists {
        return Ok(false);
    }

    let children = node.children();

    // AST malformed: a quantifier must always have at least two children
    debug_assert!(
        children.len() >= 2,
        "Quantifier node must have at least two children: TypedList and body"
    );

    // The first child is the TypedList of quantified variables
    let vars_node_id = children[0];
    let vars_node = expr.try_node(vars_node_id)?;
    debug_assert!(
        vars_node.kind() == ExprKind::TypedList,
        "First child of a quantifier must be a TypedList"
    );

    // Check if the TypedList is empty
    if vars_node.children().is_empty() {
        // The body of the quantifier is the second child
        let body_id = children[1];

        // Take ownership of the body's kind, content, and children
        let (kind, content, body_children) = {
            let body = expr.try_node_mut(body_id)?;
            (
                body.kind(),
                std::mem::take(body.content_mut()),
                std::mem::take(body.children_mut()),
            )
        };

        // Replace the quantifier node with its body
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(kind);
        node_mut.set_content(content);
        node_mut.set_children(body_children);

        return Ok(true);
    }

    Ok(false)
}

/// Fuses nested quantifiers of the same kind (forall or exists) into a single expression.
///
/// This function merges a quantifier expression with its immediate child quantifier
/// of the same type. The variables from both quantifiers are concatenated into
/// the outer quantifier's variable list. The body of the inner quantifier replaces
/// the body of the outer quantifier.
///
/// Only immediate nested quantifiers of the same type are fused. If the outer expression
/// is not a `forall` or `exists`, or if the inner quantifier is of a different kind,
/// no changes are made.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the outer quantifier expression.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if fusion completes successfully or no fusion is applicable.
/// - `Err(ExprError)` if accessing nodes or mutating the expression fails.
///
/// # Panics (in debug mode)
/// Will panic if the structure of the expressions is invalid:
/// - Outer quantifier must have at least two children: TypedList + body.
/// - First child must be a TypedList.
/// - Inner quantifier must also have at least two children with a TypedList as first child.
#[allow(dead_code)]
fn fuse_nested_quantifiers(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;

    // Only process Forall or Exists
    if node.kind() != ExprKind::Forall && node.kind() != ExprKind::Exists {
        return Ok(());
    }

    let children = node.children();

    // Outer quantifier must have at least 2 children: TypedList + body
    debug_assert!(
        children.len() >= 2,
        "Outer quantifier must have at least two children: TypedList and body"
    );

    let vars_node_id = children[0];
    let body_id = children[1];
    let vars_node = expr.try_node(vars_node_id)?;
    let body = expr.try_node(body_id)?;

    debug_assert!(
        vars_node.kind() == ExprKind::TypedList,
        "First child of outer quantifier must be a TypedList"
    );

    // Proceed only if the inner node is a quantifier of the same kind and has at least 2 children
    if body.kind() != node.kind() || body.children().len() < 2 {
        return Ok(());
    }

    let inner_vars_node_id = body.children()[0];
    let inner_body_id = body.children()[1];
    let inner_vars_node = expr.try_node(inner_vars_node_id)?;

    debug_assert!(
        inner_vars_node.kind() == ExprKind::TypedList,
        "First child of inner quantifier must be a TypedList"
    );

    // Move children of inner TypedList into outer TypedList
    let inner_children_ids = {
        let inner_vars_node_mut = expr.try_node_mut(inner_vars_node_id)?;
        std::mem::take(inner_vars_node_mut.children_mut())
    };
    if !inner_children_ids.is_empty() {
        let vars_node_mut = expr.try_node_mut(vars_node_id)?;
        for child_id in inner_children_ids {
            vars_node_mut.add_child(child_id);
        }
    }

    // Replace the body of the outer quantifier with the body of the inner quantifier
    {
        let inner_body_node_mut = expr.try_node_mut(inner_body_id)?;
        let kind_new = inner_body_node_mut.kind();
        let content_new = std::mem::take(inner_body_node_mut.content_mut());
        let children_new = std::mem::take(inner_body_node_mut.children_mut());

        let body_node_mut = expr.try_node_mut(body_id)?;
        body_node_mut.set_kind(kind_new);
        body_node_mut.set_content(content_new);
        body_node_mut.set_children(children_new);
    }

    Ok(())
}

/// Simplifies quantified nodes whose body is trivially true or false.
///
/// Rules:
/// - (forall (...) (and)) → (and)        [true]
/// - (forall (...) (or))  → (or)         [false]
/// - (exists (...) (and)) → (and)        [true]
/// - (exists (...) (or))  → (or)         [false]
///
/// Returns `true` if the node was simplified, `false` otherwise.
#[allow(dead_code)]
fn simplify_quantifier_trivial_body(
    node_id: NodeId,
    expr: &mut Expr
) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    let kind = node.kind();
    if node.kind() != ExprKind::Forall && node.kind() != ExprKind::Exists {
        return Ok(false);
    }
    let children = node.children();

    debug_assert!(
        children.len() == 2,
        "Quantifier must have exactly 2 children"
    );

    let body_id = children[1];
    let body = expr.try_node(body_id)?;

    // Check if the body is trivial (empty AND or OR)
    let trivial = body.children().is_empty();

    if trivial {
        match (kind, body.kind()) {
            (ExprKind::Forall, ExprKind::And)
            | (ExprKind::Exists, ExprKind::And)
            | (ExprKind::Forall, ExprKind::Or)
            | (ExprKind::Exists, ExprKind::Or) => {
                // Replace the quantifier by the trivial body
                let body_mut = expr.try_node_mut(body_id)?;
                let kind_new = body_mut.kind();
                let content_new = std::mem::take(body_mut.content_mut());
                let children_new = std::mem::take(body_mut.children_mut());

                let node_mut = expr.try_node_mut(node_id)?;
                node_mut.set_kind(kind_new);
                node_mut.set_content(content_new);
                node_mut.set_children(children_new);

                return Ok(true);
            }
            _ => {}
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::lir::expr::transform::simplify::quantifier::simplify_quantifier;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test that an empty forall quantifier is replaced by its body.
    /// Input: (forall () (A))
    /// Expected: (A)
    #[test]
    fn test_remove_empty_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let empty_vars = builder.typed_list(vec![]);
        let forall_node = builder.forall(empty_vars, atomic_a);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        simplify_quantifier(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Test that nested forall quantifiers are fused.
    /// Input: (forall (?X) (forall (?Y) (A)))
    /// Expected: (forall (?X ?Y) (A))
    #[test]
    fn test_fuse_nested_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);

        let y = builder.variable("?Y");
        let inner_vars = builder.typed_list(vec![y]);
        let inner_forall = builder.forall(inner_vars, atomic_a);

        let x = builder.variable("?X");
        let outer_vars = builder.typed_list(vec![x]);
        let outer_forall = builder.forall(outer_vars, inner_forall);

        builder.set_root(outer_forall).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        simplify_quantifier(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        assert_eq!(output, "(forall (?X ?Y) (A))");
    }

    /// Test that a quantifier with trivial body is replaced by its body.
    /// Input: (forall (?X) (and))
    /// Expected: (and)
    #[test]
    fn test_trivial_body_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, empty_and);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        simplify_quantifier(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and)");
    }

    /// Test that no simplification is applied when quantifier is non-empty, non-nested, non-trivial.
    /// Input: (forall (?X) (A))
    /// Expected unchanged
    #[test]
    fn test_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, atomic_a);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string(&interner);
        simplify_quantifier(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        assert_eq!(output, "(forall (?X) (A))");
    }

    /// Test that an empty exists quantifier is replaced by its body.
    /// Input: (exists () (A))
    /// Expected: (A)
    #[test]
    fn test_remove_empty_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let empty_vars = builder.typed_list(vec![]);
        let exists_node = builder.exists(empty_vars, atomic_a);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        simplify_quantifier(root_id, &mut expr).unwrap();
        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(expr.to_syntax_string(&interner), "(A)");
    }

    /// Test that nested exists quantifiers are fused into a single node.
    /// Input: (exists (?X) (exists (?Y) (A)))
    /// Expected: (exists (?X ?Y) (A))
    #[test]
    fn test_fuse_nested_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let y = builder.variable("?Y");
        let inner_vars = builder.typed_list(vec![y]);
        let inner_exists = builder.exists(inner_vars, atomic_a);

        let x = builder.variable("?X");
        let outer_vars = builder.typed_list(vec![x]);
        let outer_exists = builder.exists(outer_vars, inner_exists);

        builder.set_root(outer_exists).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        simplify_quantifier(root_id, &mut expr).unwrap();
        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(expr.to_syntax_string(&interner), "(exists (?X ?Y) (A))");
    }

    /// Test that an exists quantifier with a trivial body is replaced by its body.
    /// Input: (exists (?X) (and))
    /// Expected: (and)
    #[test]
    fn test_trivial_body_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let exists_node = builder.exists(vars, empty_and);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        simplify_quantifier(root_id, &mut expr).unwrap();
        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(expr.to_syntax_string(&interner), "(and)");
    }

    /// Test that no simplification is applied on a non-empty, non-nested exists quantifier.
    /// Input: (exists (?X) (A))
    /// Expected unchanged: (exists (?X) (A))
    #[test]
    fn test_no_simplification_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let exists_node = builder.exists(vars, atomic_a);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        simplify_quantifier(root_id, &mut expr).unwrap();
        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(expr.to_syntax_string(&interner), "(exists (?X) (A))");
    }
}
