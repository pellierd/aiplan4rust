use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::{Expr, ExprKind};
use crate::aiplan4rust::tree::NodeId;

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
/// - `logic`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(())` if logic succeeds or if no simplification is applicable.
/// - `Err(ExprError)` if any node cannot be accessed or mutated.
pub fn simplify(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    // ÉTAPE 1 : Double Négation (Structurelle)
    // (not (not X)) -> X
    if simplify_double_negation(node_id, expr)? {
        // En post-order, le nouveau contenu de node_id a déjà été simplifié.
        // On peut s'arrêter ici.
        return Ok(());
    }

    // ÉTAPE 2 : Constantes triviales (Nettoyage)
    // (not true) -> false, (not false) -> true
    if simplify_trivial_constant(node_id, expr)? {
        return Ok(());
    }

    Ok(())
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
/// - `node_id`: The `NodeId` of the node to simplification. Must be a `Not` node.
/// - `logic`: Mutable reference to the expression tree containing the node.
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
fn simplify_double_negation(
    node_id: NodeId,
    expr: &mut Expr, // Inutilisé ici mais conservé pour la signature
) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");

    let children = node.children();
    if children.is_empty() {
        return Ok(false);
    }

    let child_id = children[0];
    let child = expr.try_node(child_id)?;

    // Si l'enfant n'est pas un NOT, on ne peut pas faire de double négation
    if child.kind() != ExprKind::Not {
        return Ok(false);
    }

    let grandchild_children = child.children();
    if grandchild_children.is_empty() {
        return Ok(false);
    }

    let grandchild_id = grandchild_children[0];

    // Utilisation de move_to : on déplace le contenu du petit-enfant (X)
    // directement dans le nœud actuel (le premier NOT).
    // Cela élimine les deux couches de NOT d'un coup.
    expr.move_to(grandchild_id, node_id)?;

    Ok(true)
}

/// Simplifies trivial constant logic under a `Not` node.
///
/// This function detects cases where a `Not` node has as its single child
/// an empty `And` or `Or` expression, and flips it according to logical
/// identities:
///
/// - `(not (and))` → `(or)`
/// - `(not (or))`  → `(and)`
///
/// # Parameters
/// - `node_id`: The `NodeId` of the `Not` node to simplification.
/// - `logic`: A mutable reference to the expression tree.
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
/// - This function does **not** recursively simplification; it is intended to be
///   called during a post-order traversal.
/// - After simplification, the node is no longer a `Not`. The caller must
///   avoid applying further `Not`-specific rules to it.
fn simplify_trivial_constant(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");
    debug_assert!(
        node.children().len() == 1,
        "Not node must have exactly one child"
    );

    let child_id = node.children()[0];
    let child = expr.try_node(child_id)?;

    // On ne simplifie que si l'enfant est un And/Or vide (constante logique)
    if !child.children().is_empty() {
        return Ok(false);
    }

    match child.kind() {
        // (not true) -> false
        ExprKind::And => {
            expr.set_to_bool(node_id, false)?;
            Ok(true)
        }
        // (not false) -> true
        ExprKind::Or => {
            expr.set_to_bool(node_id, true)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::old::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::old::expr::ExprKind;

    /// Test simplification of a simple double negation: (not (not (A))) → (A)
    #[test]
    fn test_double_negation_simple_predicate() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (not (A)))
        let atomic_a = builder.atomic_formula(1, vec![]); // "A"
        let not1 = builder.not(atomic_a);
        let root = builder.not(not1);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify !!A -> A
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le nœud racine n'est plus un NOT, mais directement l'AtomicFormula
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test simplification of a nested double negation containing a subtree.
    /// Input: (not (not (and (A) (B)))) -> (and (A) (B))
    #[test]
    fn test_double_negation_with_subtree() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (not (and (A) (B))))
        let atomic_a = builder.atomic_formula(1, vec![]);
        let atomic_b = builder.atomic_formula(2, vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let not1 = builder.not(and);
        let root = builder.not(not1);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify !!Subtree -> Subtree
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // La racine doit maintenant être le nœud AND original
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }

    /// Test that no simplification is applied when negation is not doubled.
    /// Input: (not (and (A) (B))) -> unchanged
    #[test]
    fn test_not_node_no_simplification() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (and (A) (B)))
        let atomic_a = builder.atomic_formula(1, vec![]);
        let atomic_b = builder.atomic_formula(2, vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let root = builder.not(and);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify
        // On vérifie qu'une simple négation sans double négation reste intacte.
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le nœud doit rester de typing Not
        assert_eq!(root_node.kind(), ExprKind::Not);

        Ok(())
    }

    /// Test simplification of `(not (and))` -> `(or)`
    #[test]
    fn test_not_empty_and_becomes_or() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (and)) -> ¬True
        let empty_and = builder.and(vec![]);
        let root = builder.not(empty_and);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify ¬True -> False
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le résultat doit être un (or) vide
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test simplification of `(not (or))` -> `(and)`
    #[test]
    fn test_not_empty_or_becomes_and() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (or)) -> ¬False
        let empty_or = builder.or(vec![]);
        let root = builder.not(empty_or);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify ¬False -> True
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le résultat doit être un (and) vide
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that no simplification is applied on a NOT whose child is not a NOT or empty AND/OR.
    /// Input: (not (A)) -> unchanged
    #[test]
    fn test_not_other_operator_no_simplification() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (not (A))
        let atomic_a = builder.atomic_formula(1, vec![]); // "A"
        let root = builder.not(atomic_a);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify
        // On s'assure qu'un littéral négatif n'est pas touché.
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Le Kind doit rester ExprKind::Not
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(root_node.children().len(), 1);

        Ok(())
    }
}
