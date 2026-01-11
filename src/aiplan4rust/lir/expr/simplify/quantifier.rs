use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind};
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

/// Canonicalizes the variables of a quantifier node.
///
/// This function operates on the content of a `Forall` or `Exists` node,
/// which is expected to contain a `QuantifierVariables(TypedList)` holding
/// all quantified variables. Each `TypedSymbol` in the `TypedList` is sorted
/// by name to produce a canonical ordering.
///
/// # Parameters
/// - `node_id`: The ID of the quantifier node (`Forall` or `Exists`) to process.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the operation succeeds.
/// - `Err(ExprError)` if the node does not contain quantifier variables.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The node is not a quantifier (`Forall` or `Exists`).
///
/// # Notes
/// - Only the `TypedList` in the node content is affected; the body of the quantifier is untouched.
/// - Ensures that different representations of the same variables have a canonical order.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// canonicalize_quantifier_vars(node_id, &mut expr)?;
/// ```
pub fn canonicalize_quantifier_vars(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node_mut(node_id)?;

    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

    // Borrow the QuantifierVariables or return an error
    let mut vars = node.content_mut().try_quantifier_vars_mut()?;

    // Sort the TypedSymbols by name for canonical order
    vars.sort_by_key(|ts| ts.symbol());

    Ok(())
}

/// Simplifies a quantifier node by removing it if its variable list is empty.
///
/// This function operates on nodes of kind `Forall` or `Exists`. Quantified
/// variables are stored directly in the node content as
/// `ExprContent::QuantifierVariables(TypedList)`.
///
/// If the `TypedList` of quantified variables is empty, the quantifier is
/// removed and replaced by its body. The body of the quantifier is assumed
/// to have already been simplified.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node (`Forall` or `Exists`) to simplify.
/// - `expr`: A mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(true)` if the quantifier was removed and replaced by its body.
/// - `Ok(false)` if the quantifier has at least one bound variable.
/// - `Err(ExprError)` if accessing or mutating the expression tree fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if:
/// - The node is not a quantifier (`Forall` or `Exists`).
/// - The node content is not `QuantifierVariables`.
fn remove_empty_quantifier(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node_mut(node_id)?;

    debug_assert!(
        node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
        "Node must be a quantifier (Forall or Exists)"
    );

    // Borrow the TypedList of quantifier variables or return an error
    let vars = node.content_mut().try_quantifier_vars()?;

    if vars.is_empty() {
        // Replace the quantifier with its body (assumes exactly one child: the body)
        let body_id = node.children()[0];
        expr.move_to(body_id, node_id)?;
        return Ok(true);
    }

    Ok(false)
}

/// Fuses immediate nested quantifiers of the same kind (`forall` or `exists`) into a single node.
///
/// This function merges a quantifier with its direct child quantifier of the same type.
/// The variables from the inner quantifier are moved into the outer quantifier's
/// `QuantifierVariables` content. The body of the inner quantifier replaces the
/// body of the outer quantifier.
///
/// Only immediate nested quantifiers of the same type are fused. If the outer node
/// is not a quantifier, or if the inner quantifier is of a different kind, no changes are made.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the outer quantifier node.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if fusion completes successfully or no fusion is applicable.
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Panics (in debug mode)
/// The function contains `debug_assert!` checks that will panic if the AST structure is invalid:
/// - The outer quantifier must have exactly one child: the body.
/// - If the inner quantifier has the same kind as the outer, it must also have exactly one child: the body.
/// - These assertions ensure that the inner quantifier's structure is valid before merging.
///
/// # Notes
/// - Variables from the inner quantifier are moved (not cloned) into the outer quantifier.
/// - The body of the inner quantifier replaces the body of the outer quantifier.
/// - Inner quantifiers of a different kind are ignored.
/// - Only immediate nested quantifiers are fused; deeper nesting is not handled recursively.

pub fn fuse_nested_quantifiers(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // Step 1: read outer node kind and children immutably
    let outer_node = expr.try_node(node_id)?;
    let outer_kind = outer_node.kind();
    debug_assert!(
        outer_kind == ExprKind::Forall || outer_kind == ExprKind::Exists,
        "Outer node must be a quantifier (Forall or Exists)"
    );
    let outer_children = outer_node.children();
    debug_assert!(outer_children.len() == 1, "Outer quantifier must have exactly one child");
    let inner_id = outer_children[0];

    // Step 2: read inner node kind immutably
    let inner_kind = expr.try_node(inner_id)?.kind();
    if inner_kind != outer_kind {
        return Ok(()); // Different kinds, skip fusion
    }

    // Step 3: take inner variables mutably
    let mut inner_vars = {
        let inner_node = expr.try_node_mut(inner_id)?;
        std::mem::take(inner_node.content_mut().try_quantifier_vars_mut()?)
    };

    // Step 4: prepend inner vars to outer vars mutably
    {
        let outer_node = expr.try_node_mut(node_id)?;
        let outer_vars = outer_node.content_mut().try_quantifier_vars_mut()?;
        outer_vars.splice(0..0, inner_vars.into_iter());
        outer_vars.sort_by_key(|v| v.symbol());
        outer_vars.dedup_by_key(|v| v.symbol());
    }

    // Step 5: replace outer body with inner body
    let inner_body_id = expr.try_node(inner_id)?.children()[0];
    {
        let inner_body_node = expr.try_node_mut(inner_body_id)?;
        let kind_new = inner_body_node.kind();
        let content_new = std::mem::take(inner_body_node.content_mut());
        let children_new = std::mem::take(inner_body_node.children_mut());
        expr.set(inner_id, kind_new, content_new, children_new)?;
    }

    Ok(())
}

/// Simplifies quantified nodes whose body is trivially true or false.
///
/// This function handles quantifier nodes (`forall` or `exists`) whose body is an
/// empty logical conjunction (`and`) or disjunction (`or`). The simplification rules are:
/// - `(forall (...) (and))` → `(and)`
/// - `(forall (...) (or))`  → `(or)`
/// - `(exists (...) (and))` → `(and)`
/// - `(exists (...) (or))`  → `(or)`
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
/// - The quantifier does not have exactly one child (the body).
///
/// # Notes
/// - Only empty `and` or `or` bodies are considered trivial.
/// - The variables of the quantifier are stored in the node's `Content` and are ignored
///   during this simplification.
/// - The body of the quantifier completely replaces the quantifier node.
/// - Non-trivial bodies are left unchanged.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// simplify_quantifier_trivial_body(node_id, &mut expr)?;
/// ```
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
        children.len() == 1,
        "Quantifier must have exactly 1 child"
    );

    let body_id = children[0];
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
    use crate::aiplan4rust::lang::TypedList;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::lir::expr::simplify::quantifier;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

    /// Test that an empty forall quantifier is replaced by its body.
    /// Input: (forall () (A))
    /// Expected: (A)
    #[test]
    fn test_remove_empty_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let forall_node = builder.forall(TypedList::new(), atomic_a);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Test that nested forall quantifiers are fused.
    /// Input: (forall (?X - T2) (forall (?Y - T1) (A)))
    /// Expected: (forall (?X - T2 ?Y - T1) (A))
    #[test]
    fn test_fuse_nested_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);

        let inner_forall = builder.forall_with_string_vars(vec![("?Y", "T1")], atomic_a);
        let outer_forall = builder.forall_with_string_vars(vec![("?X", "T2")], inner_forall);

        builder.set_root(outer_forall).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        assert_eq!(output, "(forall (?X - T2 ?Y - T1) (A))");
    }

    /// Test that a quantifier with trivial body is replaced by its body.
    /// Input: (forall (?X - T) (and))
    /// Expected: (and)
    #[test]
    fn test_trivial_body_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let forall_node = builder.forall_with_string_vars(vec![("?X", "T")], empty_and);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and)");
    }

    /// Test that no simplification is applied when quantifier is non-empty, non-nested, non-trivial.
    /// Input: (forall (?X - T) (A))
    /// Expected unchanged
    #[test]
    fn test_no_simplification_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let forall_node = builder.forall_with_string_vars(vec![("?X", "T")], atomic_a);

        builder.set_root(forall_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        assert_eq!(output, "(forall (?X - T) (A))");
    }

    /// Test that an empty exists quantifier is replaced by its body.
    /// Input: (exists () (A))
    /// Expected: (A)
    #[test]
    fn test_remove_empty_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let exists_node = builder.exists_with_string_vars(vec![], atomic_a);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Test that nested exists quantifiers are fused into a single node.
    /// Input: (exists (?X - T2) (exists (?Y - T1) (A)))
    /// Expected: (exists (?Y - T1 ?X - T2) (A))
    #[test]
    fn test_fuse_nested_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let inner_exists = builder.exists_with_string_vars(vec![("?Y", "T1")], atomic_a);
        let outer_exists = builder.exists_with_string_vars(vec![("?X", "T2")], inner_exists);

        builder.set_root(outer_exists).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(output, "(exists (?X - T2 ?Y - T1) (A))");
    }


    /// Test that an exists quantifier with a trivial body is replaced by its body.
    /// Input: (exists (?X - T) (and))
    /// Expected: (and)
    #[test]
    fn test_trivial_body_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let exists_node = builder.exists_with_string_vars(vec![("?X", "T")], empty_and);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and)");
    }

    /// Test that no simplification is applied on a non-empty, non-nested exists quantifier.
    /// Input: (exists (?X - T) (A))
    /// Expected unchanged: (exists (?X - T) (A))
    #[test]
    fn test_no_simplification_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let exists_node = builder.exists_with_string_vars(vec![("?X", "T")], atomic_a);

        builder.set_root(exists_node).unwrap();
        let mut expr = builder.finish();
        let root_id = expr.root_id().unwrap();

        let input = expr.to_syntax_string_with_interner(&interner);
        quantifier::simplify(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        assert_eq!(output, "(exists (?X - T) (A))");
    }

}
