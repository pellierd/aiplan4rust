use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode};

/// Simplifies a quantifier node (`forall` or `exists`) by applying a sequence of transformations.
///
/// This function applies the following steps in order:
/// 1. Canonicalizes the variables in the `TypedList` of the quantifier.
/// 2. Removes the quantifier if the variable list is empty, replacing it with its body.
/// 3. Fuses nested quantifiers of the same kind, combining their variable lists.
/// 4. Simplifies the quantifier if its body is trivially true or false (e.g., `(forall (x) (and))` → `(and)`).
///
/// Each step is applied only if applicable. The first step that modifies the expression may
/// terminate the normalization early if the node is replaced or simplified.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node to normalize.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if normalization completes successfully (even if no changes were made).
/// - `Err(ExprError)` if any step fails to access or modify nodes.
///
/// # Panics (in debug mode)
/// Panics are triggered inside the called functions if the AST is malformed:
/// - The node is not a quantifier (`Forall` or `Exists`).
/// - The quantifier does not have exactly two children (TypedList + body).
/// - The first child of the quantifier is not a `TypedList`.
/// - Inner quantifiers in `fuse_nested_quantifiers` are malformed (same kind but not exactly two children).
///
/// # Notes
/// - This function relies on the called functions (`canonicalize_quantifier_vars`,
///   `remove_empty_quantifier`, `fuse_nested_quantifiers`, `simplify_quantifier_trivial_body`)
///   to enforce structural checks via `debug_assert!`.
/// - No redundant assertions are performed here to avoid duplication.
/// - Intended to be used on nodes already known or assumed to be quantifiers.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// normalize(node_id, &mut expr)?;
/// ```
pub(super) fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
    // Step 1: canonicalize the variables in the TypedList
    canonicalize_quantifier_vars(node_id, expr)?;
    // Step 2: remove empty quantifiers if the TypedList has no variables
    if remove_empty_quantifier(node_id, expr)? {
        return Ok(());
    }
    // Step 3: fuse nested quantifiers of the same type
    fuse_nested_quantifiers(node_id, expr)?;
    // Step 4: simplify trivial body (e.g., forall (x) (and) -> (and))
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
/// - `Ok(false)` if the variable list is not empty.
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The node is not a quantifier (`forall` or `exists`).
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
    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

    let children = node.children();
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
        let body_id = children[1];
        expr.move_to(body_id, node_id)?;
        return Ok(true);
    }

    Ok(false)
}

/// Canonicalizes the variable list of a quantifier node.
///
/// This function sorts the first child of a `Forall` or `Exists` node,
/// which is expected to be a `TypedList` of quantified variables. The goal
/// is to put the variables in a canonical order to make structural comparisons
/// and simplifications more reliable.
///
/// # Parameters
/// - `node_id`: The ID of the quantifier node (`Forall` or `Exists`) to process.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the operation succeeds.
/// - `Err(ExprError)` if accessing or mutating the node fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The node is not a quantifier (`Forall` or `Exists`).
/// - The quantifier node has no children.
/// - The first child is not a `TypedList`.
///
/// # Notes
/// - Only the first child (the `TypedList`) is affected; the body of the quantifier is untouched.
/// - These assertions help ensure that `(forall (x y) ...)` and `(forall (y x) ...)`
///   have a canonical representation for deduplication and simplification.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// canonicalize_quantifier_vars(node_id, &mut expr)?;
/// ```
#[allow(dead_code)]
pub fn canonicalize_quantifier_vars(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

    let children = node.children();
    debug_assert!(
        !children.is_empty(),
        "Quantifier node must have at least one child (TypedList)"
    );

    let vars_node_id = children[0];
    let vars_node = expr.try_node_mut(vars_node_id)?;
    debug_assert!(
        vars_node.kind() == ExprKind::TypedList,
        "First child of a quantifier must be a TypedList"
    );

    let vars_children = vars_node.children_mut();
    vars_children.sort(); // canonical order: assumes NodeId implements Ord

    Ok(())
}

/// Fuses nested quantifiers of the same kind (`forall` or `exists`) into a single expression.
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
/// The function contains `debug_assert!` checks that will panic if the AST structure is invalid:
/// - The outer quantifier must have exactly two children: TypedList + body.
/// - The first child of the outer quantifier must be a `TypedList`.
/// - If the inner quantifier has the same kind as the outer, it must also have exactly two children:
///   a `TypedList` and a body. Otherwise, no fusion occurs.
/// - These assertions ensure that the inner quantifier's structure is valid before merging.
///
/// # Notes
/// - Variables from the inner quantifier are appended to the outer quantifier's variable list.
/// - The body of the inner quantifier replaces the body of the outer quantifier.
/// - Inner quantifiers of a different kind are ignored.
/// - Only immediate nested quantifiers are fused; deeper nesting is not handled recursively.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// fuse_nested_quantifiers(node_id, &mut expr)?;
/// ```
#[allow(dead_code)]
fn fuse_nested_quantifiers(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

    let children = node.children();
    debug_assert!(
        children.len() == 2,
        "Outer quantifier must have exactly two children: TypedList and body"
    );

    let vars_node_id = children[0];
    let body_id = children[1];
    let vars_node = expr.try_node(vars_node_id)?;
    let body = expr.try_node(body_id)?;

    debug_assert!(
        vars_node.kind() == ExprKind::TypedList,
        "First child of outer quantifier must be a TypedList"
    );

    // Only consider inner quantifier if same kind
    if body.kind() != node.kind() {
        return Ok(()); // different kind, skip
    }

    // Assert that the inner quantifier has exactly 2 children
    debug_assert!(
        body.children().len() == 2,
        "Inner quantifier of the same kind must have exactly 2 children: TypedList + body"
    );

    let inner_vars_node_id = body.children()[0];
    let inner_body_id = body.children()[1];

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
        expr.set(body_id, kind_new, content_new, children_new)?;
    }

    Ok(())
}

/// Simplifies quantified nodes whose body is trivially true or false.
///
/// This function handles quantifier nodes (`forall` or `exists`) whose body is an
/// empty logical conjunction (`and`) or disjunction (`or`). The simplification rules are:
/// - `(forall (...) (and))` → `(and)`        [true]
/// - `(forall (...) (or))`  → `(or)`         [false]
/// - `(exists (...) (and))` → `(and)`        [true]
/// - `(exists (...) (or))`  → `(or)`         [false]
///
/// The quantifier is replaced by its trivial body if these conditions are met.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node to simplify.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(true)` if the quantifier was simplified.
/// - `Ok(false)` if no simplification was applicable.
/// - `Err(ExprError)` if accessing or mutating nodes fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The node is not a quantifier (`Forall` or `Exists`).
/// - The quantifier does not have exactly two children (TypedList + body).
///
/// # Notes
/// - Only empty `and` or `or` bodies are considered trivial.
/// - The body of the quantifier completely replaces the quantifier node.
/// - Non-trivial bodies are left unchanged.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// simplify_quantifier_trivial_body(node_id, &mut expr)?;
/// ```
#[allow(dead_code)]
fn simplify_quantifier_trivial_body(
    node_id: NodeId,
    expr: &mut Expr
) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    let kind = node.kind();
    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

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
                let body_kind = body_mut.kind();
                let body_content = std::mem::take(body_mut.content_mut());
                let body_children = std::mem::take(body_mut.children_mut());

                // Replace node_id with body
                expr.set(node_id, body_kind, body_content, body_children)?;

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
    use crate::aiplan4rust::lir::expr::simplify::quantifier;
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
        quantifier::simplify(root_id, &mut expr).unwrap();
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
        quantifier::simplify(root_id, &mut expr).unwrap();
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
        quantifier::simplify(root_id, &mut expr).unwrap();
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
        quantifier::simplify(root_id, &mut expr).unwrap();
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

        let input = expr.to_syntax_string(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
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

        let input = expr.to_syntax_string(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(output, "(exists (?X ?Y) (A))");
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

        let input = expr.to_syntax_string(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and)");
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

        let input = expr.to_syntax_string(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(output, "(exists (?X) (A))");
    }

}
