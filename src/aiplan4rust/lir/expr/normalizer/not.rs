use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::NodeId;


/// Simplifies a `Not` node in a PDDL expression tree.
///
/// This function currently handles:
/// 1. **Double negation**: `(not (not X))` → `X`
///    - Only simplifies if the node is a `Not` with exactly one child, and that child
///      is a `Not` with exactly one child itself.
///    - `debug_assert!` statements verify these invariants in debug builds.
/// 2. **Trivial negation over empty AND/OR nodes**: `(not (and))` → `(or)`, `(not (or))` → `(and)`
///    - Only applies to `Not` nodes whose child is an empty `And` or `Or`.
///    - `debug_assert!` checks the child node structure in debug builds.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the node to normalize.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(())` if normalization succeeds or if no simplification is applicable.
/// - `Err(ExprError)` if any node cannot be accessed or mutated.
pub(crate) fn normalize(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
    if !simplify_double_negation(node_id, expr)? {
        simplify_trivial_constant(node_id, expr)?;
    }
    Ok(())
}

/// Recursively pushes negations down the expression tree using De Morgan’s laws
/// and quantifier negation rules.
///
/// # Transformations applied
/// 1. De Morgan’s laws:
///    - `Not(And(...))` → `Or(Not(...))`
///    - `Not(Or(...))` → `And(Not(...))`
/// 2. Quantifier negation rules:
///    - `Not(Forall x φ)` → `Exists x Not(φ)`
///    - `Not(Exists x φ)` → `Forall x Not(φ)`
///
/// # Parameters
/// - `root_id`: NodeId of the root of the subtree to process.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(())` if all negations are successfully pushed down.
/// - `Err(ExprError)` if accessing or modifying nodes fails.
///
/// # Notes
/// - This function mutates the tree in place.
/// - Newly created `Not` nodes are added to the stack for further processing.
/// - Assumes that each `Not` node has exactly one child; this is verified with a `debug_assert!`.
pub(crate) fn push_negations(root_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
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
            _ => {} // No action for other node types
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
/// - `node_id`: NodeId of the `Not` node to transform.
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
fn apply_quantifier_negation(node_id: NodeId, expr: &mut Expr) -> Result<NodeId, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");

    let child_id = node.children()[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(
        child.kind() == ExprKind::Forall || child.kind() == ExprKind::Exists,
        "Child of Not must be a quantifier"
    );
    debug_assert!(child.children().len() == 2, "Quantifier node must have exactly two children");

    // Copy values to avoid borrow conflicts
    let child_kind = child.kind();
    let children_of_child = child.children().to_vec();
    let var_id = children_of_child[0];
    let body_id = children_of_child[1];

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
    node_mut.set_children(vec![var_id, new_not_id]);

    Ok(new_not_id)
}

/// Simplifies a double negation in a PDDL expression tree.
///
/// This function detects the pattern `(not (not X))` and replaces the
/// outer `Not` node with the grandchild `X`, effectively removing the
/// double negation. The transformation is performed in-place using
/// `std::mem::take()` to move the kind, content, and children from the
/// grandchild node.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the node to simplify. Must be a `Not` node.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Behavior
/// - The function assumes (via `debug_assert!`) that the node is a `Not` with
///   exactly one child in debug builds.
/// - If the node is not a `Not`, or if it has zero or multiple children,
///   no simplification is applied.
/// - If the single child is not a `Not`, or if that child has zero or multiple
///   children, no simplification is applied.
/// - If the node matches the double-negation pattern `(not (not X))`,
///   the outer `Not` node is replaced by `X`.
///
/// # Returns
/// - `Ok(true)` if the double negation was detected and simplified.
/// - `Ok(false)` if no simplification applies.
/// - `Err(ExprError)` if accessing or mutating nodes fails.
///
/// # Notes
/// - This function does *not* recurse by itself; it is intended to be used
///   as part of a post-order traversal.
/// - After simplification, the current node is no longer a `Not`. The caller
///   must avoid applying further `Not`-specific rules to it.
///
/// # Example
/// ```text
/// Input:  (not (not X))
/// Output: X
/// ```
fn simplify_double_negation(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");
    debug_assert!(node.children().len() == 1, "Not node must have exactly one child");

    let children = node.children();
    let child_id = children[0];
    let child = expr.try_node(child_id)?;
    if child.kind() != ExprKind::Not {
        return Ok(false); // only simplify double negation
    }
    debug_assert!(child.children().len() == 1, "Not node must have exactly one child");

    let grandchild_id = child.children()[0];
    let grandchild_node = {
        let grandchild_mut = expr.try_node_mut(grandchild_id)?;
        let kind = grandchild_mut.kind();
        let content = std::mem::take(grandchild_mut.content_mut());
        let children = std::mem::take(grandchild_mut.children_mut());
        (kind, content, children)
    };

    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(grandchild_node.0);
    node_mut.set_content(grandchild_node.1);
    node_mut.set_children(grandchild_node.2);

    Ok(true)
}

/// Simplifies trivial constant expressions under a `Not` node.
///
/// This function detects cases where a `Not` node has as its single child
/// an empty `And` or `Or` expression, and flips it according to logical
/// identities:
///
/// - `(not (and))` → `(or)`
/// - `(not (or))`  → `(and)`
///
/// # Parameters
/// - `node_id`: The `NodeId` of the `Not` node to simplify.
/// - `expr`: A mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if a simplification was applied.
/// - `Ok(false)` if no simplification applies.
/// - `Err(ExprError)` if accessing nodes fails.
///
/// # Behavior
/// - If the node is not a `Not`, returns `Ok(false)`.
/// - A `debug_assert!` ensures that a `Not` node has exactly one child.
///   In release mode, the function safely returns `Ok(false)` if this
///   structural invariant is violated.
/// - If the child is an empty `And` or `Or`, the `Not` node is rewritten
///   into the opposite connective with no children and with content set
///   to `None`.
///
/// # Notes
/// - This function does **not** recursively simplify; it is intended to be
///   called during a post-order traversal.
/// - After simplification, the node is no longer a `Not`. The caller must
///   avoid applying further `Not`-specific rules to it.
fn simplify_trivial_constant(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    // 1. Get the node
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");
    debug_assert!(node.children().len() == 1, "Not node must have exactly one child");

    // 2. Get child id and kind before mutable borrow
    let child_id = node.children()[0];
    let child_kind;
    let child_empty;
    {
        let child = expr.try_node(child_id)?;
        child_kind = child.kind();
        child_empty = child.children().is_empty();
    }

    if !child_empty {
        return Ok(false); // only simplify empty And/Or
    }

    // 3. Mutable borrow pour modifier le Not
    let node_mut = expr.try_node_mut(node_id)?;
    match child_kind {
        ExprKind::And => node_mut.set_kind(ExprKind::Or),
        ExprKind::Or  => node_mut.set_kind(ExprKind::And),
        _ => return Ok(false),
    }
    node_mut.set_children(vec![]);
    node_mut.set_content(Content::None);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test simplification of a simple double negation: (not (not (A))) → (A)
    #[test]
    fn test_double_negation_simple_predicate() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let not1 = builder.not(atomic_a);
        let root = builder.not(not1);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Test simplification of a nested double negation containing a subtree.
    /// Input: (not (not (and (A) (B)))) -> (and (A) (B))
    #[test]
    fn test_double_negation_with_subtree() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let atomic_b = builder.atomic_formula("B", vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let not1 = builder.not(and);
        let root = builder.not(not1);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (A) (B))");
    }

    /// Test that no simplification is applied when negation is not doubled.
    /// Input: (not (and (A) (B))) -> unchanged
    #[test]
    fn test_not_node_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let atomic_b = builder.atomic_formula("B", vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let root = builder.not(and);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(output, "(not (and (A) (B)))");
    }

    /// Test simplification of `(not (and))` -> `(or)`
    #[test]
    fn test_not_empty_and_becomes_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let root = builder.not(empty_and);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Test simplification of `(not (or))` -> `(and)`
    #[test]
    fn test_not_empty_or_becomes_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let root = builder.not(empty_or);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test that no simplification is applied on a NOT whose child is not a NOT or empty AND/OR.
    /// Input: (not (A)) -> unchanged
    #[test]
    fn test_not_other_operator_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let root = builder.not(atomic_a);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(output, "(not (A))");
    }

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

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

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

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

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
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, a);
        let root = builder.not(forall_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Exists);
        let children = root_node.children();
        assert_eq!(children.len(), 2);
        let var_list = expr.try_node(children[0]).unwrap();
        assert_eq!(var_list.kind(), ExprKind::TypedList);
        let body_node = expr.try_node(children[1]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(exists (?X) (not (A)))");
    }

    /// Test pushing negation through an Exists quantifier.
    /// Input: (not (exists x (A))) -> (forall x (not (A)))
    #[test]
    fn test_push_negation_exists() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let exists_node = builder.exists(vars, a);
        let root = builder.not(exists_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        let children = root_node.children();
        assert_eq!(children.len(), 2);
        let var_list = expr.try_node(children[0]).unwrap();
        assert_eq!(var_list.kind(), ExprKind::TypedList);
        let body_node = expr.try_node(children[1]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(forall (?X) (not (A)))");
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

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

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
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let exists_c = builder.exists(vars, c);
        let and_node = builder.and(vec![a, not_b, exists_c]);
        let root = builder.not(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

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
        assert_eq!(third.children().len(), 2);
        let vars_node = expr.try_node(third.children()[0]).unwrap();
        assert_eq!(vars_node.kind(), ExprKind::TypedList);
        let body_node = expr.try_node(third.children()[1]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Not);
        assert_eq!(output, "(or (not (A)) (not (not (B))) (forall (?X) (not (C))))");
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

        let y = builder.variable("?Y");
        let vars_y = builder.typed_list(vec![y]);
        let exists_d = builder.exists(vars_y, d);

        let x = builder.variable("?X");
        let vars_x = builder.typed_list(vec![x]);
        let forall_exists_d = builder.forall(vars_x, exists_d);

        let and_node = builder.and(vec![a, not_or_bc, forall_exists_d]);
        let root = builder.not(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_negations(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

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
        assert_eq!(third.children().len(), 2);
        let vars_node = expr.try_node(third.children()[0]).unwrap();
        assert_eq!(vars_node.kind(), ExprKind::TypedList);
        let body_node = expr.try_node(third.children()[1]).unwrap();
        assert_eq!(body_node.kind(), ExprKind::Forall);
        assert_eq!(body_node.children().len(), 2);
        let inner_vars = expr.try_node(body_node.children()[0]).unwrap();
        assert_eq!(inner_vars.kind(), ExprKind::TypedList);
        let inner_body = expr.try_node(body_node.children()[1]).unwrap();
        assert_eq!(inner_body.kind(), ExprKind::Not);
        assert_eq!(
            output,
            "(or (not (A)) (not (not (or (B) (C)))) (exists (?X) (forall (?Y) (not (D)))))"
        );
    }
}
