use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::tree::{NodeId, SyntaxContent};

/// Simplifies an FComp node by applying all relevant normalizations and simplifications.
///
/// This function performs the following transformations in order:
///
/// 1. **Normalization**
///    Converts asymmetric comparisons into a canonical ordering:
///    - (> a b)  → (< b a)
///    - (>= a b) → (<= b a)
///
/// 2. **Canonization**
///    Ensures a unique structural ordering for commutative comparisons:
///    - (= b a) → (= a b)
///
/// 3. **Constant evaluation**
///    Evaluates comparisons where both children are numeric constants:
///    - (= 3 3) → `and`
///    - (< 5 2) → `or`
///
/// 4. **Identity simplification**
///    Handles structurally identical operands:
///    - (= x x) or (<= x x) → `and`
///    - (< x x) or (> x x)  → `or`
///
/// # Arguments
///
/// * `node_id` - The ID of the FComp node.
/// * `expr` - The expression tree containing the node.
///
/// # Returns
///
/// * `Err(ExprError)` on structural access issues.
pub fn simplify(
    node_id: NodeId,
    expr: &mut Expr
) -> Result<(), LogicError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::FComp {
        return Ok(());
    }

    // Step 1: Normalize asymmetric comparisons (> → <, >= → <=)
    normalize_comparison(node_id, expr)?;

    // Step 2: Canonicalize commutative comparisons (=)
    canonicalize_comparison(node_id, expr)?;

    // Step 3: Evaluate constant comparisons
    if simplify_comparison_constants(node_id, expr)? {
        return Ok(()); // Node replaced → further steps irrelevant
    }

    // Step 4: Simplify trivial identities (x = x, x < x, ...)
    if simplify_comparison_trivial_identity(node_id, expr)? {
        return Ok(());
    }

    Ok(())
}


/// Normalize asymmetric comparisons in FComp nodes:
///   (> a b)  → (< b a)
///   (>= a b) → (<= b a)
///
/// Returns Ok(true) if modified.
fn normalize_comparison(node_id: NodeId, expr: &mut Expr) -> Result<bool, LogicError> {
    let node = expr.try_node(node_id)?;

    // We only handle FComp nodes
    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let op = match node.content().as_binary_comp() {
        Some(op) => op,
        None => return Ok(false), // should not happen normally
    };

    // Only > and >= need expr
    let new_op = match op {
        BinaryComp::Greater => Some(BinaryComp::Less),
        BinaryComp::GreaterEq => Some(BinaryComp::LessEq),
        _ => None,
    };

    if new_op.is_none() {
        return Ok(false);
    }

    // Must be binary
    let children = node.children();
    if children.len() != 2 {
        debug_assert!(false, "FComp node should have 2 children");
        return Ok(false);
    }

    let left = children[0];
    let right = children[1];

    // Apply modification
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_content(Content::BinaryComp(new_op.unwrap()));
    node_mut.set_children(vec![right, left]); // swap operands

    Ok(true)
}

/// Canonicalize commutative equality (=):
///   (= b a) → (= a b)
///
/// Returns Ok(true) if reordered.
fn canonicalize_comparison(node_id: NodeId, expr: &mut Expr) -> Result<bool, LogicError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let op = match node.content().as_binary_comp() {
        Some(op) => op,
        None => return Ok(false),
    };

    // Only = is commutative
    if op != BinaryComp::Equal {
        return Ok(false);
    }

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(false, "FComp node should have 2 children");
        return Ok(false);
    }

    let a = children[0];
    let b = children[1];

    // Canonical order based on NodeId (or any other stable metric)
    if b < a {
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_children(vec![b, a]);
        return Ok(true);
    }

    Ok(false)
}

/// Simplifies a `Comparison` node if both operands are constant numeric values.
///
/// This function evaluates a binary comparison (`BinaryComp`) between two constant numbers
/// (`Float` nodes). If both children of the node are constants, it replaces the `Comparison`
/// node with:
/// - `ExprKind::And` if the comparison evaluates to `true` (always satisfied),
/// - `ExprKind::Or` if the comparison evaluates to `false` (never satisfied).
///
/// # Parameters
///
/// * `node_id` - The ID of the comparison node to simplify.
/// * `expr` - A mutable reference to the expression tree (`Expr`) containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was successfully simplified.
/// * `Ok(false)` if the node could not be simplified (wrong kind, missing `BinaryComp` content,
///    wrong number of children, or non-constant children).
/// * `Err(ExprError)` if node access or mutation fails.
///
/// # Notes
///
/// - This function only operates on nodes of kind `ExprKind::FComp`.
/// - The simplification is safe and deterministic because it only evaluates constant values.
/// - After simplification, the node has no children and its content is set to `Content::None`.
fn simplify_comparison_constants(node_id: NodeId, expr: &mut Expr) -> Result<bool, LogicError> {
    let node = expr.try_node(node_id)?;

    // Ensure the node is of type FComp
    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let op = match node.content().as_binary_comp() {
        Some(op) => op,
        None => {
            debug_assert!(false, "FComp node without BinaryComp content");
            return Ok(false); // should not happen in normal usage
        }
    };

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(children.len() == 2, "FComp node does not have exactly 2 children");
        return Ok(false);
    }

    // Try to get constant values from both children
    let left_val = match expr.try_node(children[0])?.content().as_float() {
        Some(v) => v,
        None => return Ok(false),
    };
    let right_val = match expr.try_node(children[1])?.content().as_float() {
        Some(v) => v,
        None => return Ok(false),
    };

    // Evaluate the comparison directly on OrderedFloat
    let result = match op {
        BinaryComp::Equal => left_val == right_val,
        BinaryComp::Greater => left_val > right_val,
        BinaryComp::Less => left_val < right_val,
        BinaryComp::GreaterEq => left_val >= right_val,
        BinaryComp::LessEq => left_val <= right_val,
    };

    // Replace the node using set_to
    expr.set_to(node_id, result)?;

    Ok(true)
}

/// Simplifies an FComp node when both children are trivially identical.
///
/// This function handles comparisons where the left and right children are exactly the same:
/// - `= x x` or `= f(x) f(x)` → always true → replaced with `and`
/// - `>= x x` or `<= x x` → always true → replaced with `and`
/// - `< x x` or `> x x` → always false → replaced with `or`
///
/// # Arguments
///
/// * `node_id` - The ID of the FComp node to simplify.
/// * `expr` - Mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was simplified.
/// * `Ok(false)` if no simplification was possible (different terms, not FComp, etc.).
/// * `Err(ExprError)` if accessing or mutating the node fails.
/// Simplifies an FComp node when both children are structurally identical.
///
/// This function handles comparisons where the left and right children are exactly the same,
/// either trivially or structurally:
/// - `= x x` or `= f(x) f(x)` → always true → replaced with `and`
/// - `>= x x` or `<= x x` → always true → replaced with `and`
/// - `< x x` or `> x x` → always false → replaced with `or`
///
/// # Arguments
///
/// * `node_id` - The ID of the FComp node to simplify.
/// * `expr` - Mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was simplified.
/// * `Ok(false)` if no simplification was possible (different terms, not FComp, etc.).
/// * `Err(ExprError)` if accessing or mutating the node fails.
fn simplify_comparison_trivial_identity(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, LogicError> {
    let node = expr.try_node(node_id)?;

    // Only operate on FComp nodes
    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(children.len() == 2, "FComp node does not have exactly 2 children");
        return Ok(false);
    }

    let left_id = children[0];
    let right_id = children[1];

    // Use deep_sub_expr_eq to check if the two children are structurally identical
    if expr.deep_sub_expr_eq(left_id, right_id)? {
        let op = node.content().as_binary_comp();
        let is_true = matches!(
            op,
            Some(BinaryComp::Equal)
                | Some(BinaryComp::GreaterEq)
                | Some(BinaryComp::LessEq)
        );

        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(if is_true { ExprKind::And } else { ExprKind::Or });
        node_mut.set_content(Content::None);
        node_mut.set_children(vec![]);
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;

    /// Test that a constant equality comparison is simplified to `and`.
    /// Input: (= 3 3)
    /// Expected: (and)
    #[test]
    fn test_simplify_constant_equal() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= 3.0 3.0)
        let left = builder.number(3.0);
        let right = builder.number(3.0);
        let eq_node = builder.equal(left, right);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= x x) -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // L'égalité disparait au profit d'un (and) vide (True)
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that a constant inequality comparison is simplified to `or`.
    /// Input: (= 2 3)
    /// Expected: (or)
    #[test]
    fn test_simplify_constant_not_equal() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= 2.0 3.0)
        let left = builder.number(2.0);
        let right = builder.number(3.0);
        let eq_node = builder.equal(left, right);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= 2 3) -> False
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // L'égalité est remplacée par un (or) vide, signifiant "Faux"
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that a comparison with identical variables simplifies correctly.
    /// Input: (= ?x ?x)
    /// Expected: (and)
    #[test]
    fn test_simplify_identical_variables() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= ?x ?x)
        let x = builder.variable(1); // "?x"
        let eq_node = builder.equal(x, x);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= ?x ?x) -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // On s'assure que même sans connaître la valeur de ?x,
        // le moteur reconnaît que c'est une tautologie.
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that a comparison with structurally identical functions simplifies correctly.
    /// Input: (= (f a b) (f a b))
    /// Expected: (and)
    #[test]
    fn test_simplify_structural_identity() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= (f ?a ?b) (f ?a ?b))
        let a = builder.variable(1);
        let b = builder.variable(2);
        // f1 et f2 sont deux nœuds différents dans l'AST au départ
        let f1 = builder.function_term(3, vec![a, b]);
        let f2 = builder.function_term(3, vec![a, b]);

        let eq_node = builder.equal(f1, f2);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= f1 f2) -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le moteur a identifié que f1 == f2 structurellement
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that a non-simplifiable comparison remains unchanged.
    /// Input: (= ?x ?y)
    /// Expected: (= ?x ?y)
    #[test]
    fn test_no_simplification_different_variables() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= ?x ?y)
        let x = builder.variable(1); // "?x"
        let y = builder.variable(2); // "?y"
        let eq_node = builder.equal(x, y);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (ne devrait rien changer ici)
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // On vérifie que le nœud est resté une comparaison (FComp / Equal)
        // et n'a pas été transformé en (and) ou (or)
        assert_eq!(root_node.kind(), ExprKind::FComp);
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }

    /// Test that greater-equal on identical variables simplifies to `and`.
    /// Input: (>= ?x ?x)
    /// Expected: (and)
    #[test]
    fn test_simplify_greater_eq_identity() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (>= ?x ?x)
        let x = builder.variable(1); // "?x"
        let ge_node = builder.greater_eq(x, x);

        builder.set_root(ge_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (>= ?x ?x) -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // On s'attend à ce que l'inégalité large soit simplifiée en "Vrai"
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that greater-than on identical variables simplifies to `or`.
    /// Input: (> ?x ?x)
    /// Expected: (or)
    #[test]
    fn test_simplify_greater_identity() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (> ?x ?x)
        let x = builder.variable(1);
        let gt_node = builder.greater(x, x);

        builder.set_root(gt_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (> ?x ?x) -> False
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Une valeur ne peut pas être strictement supérieure à elle-même.
        // Le résultat doit être un (or) vide.
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());

        Ok(())
    }
}
