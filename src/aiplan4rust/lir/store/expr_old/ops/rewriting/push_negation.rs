use crate::aiplan4rust::lir::store::expr_old::content::Content;
use crate::aiplan4rust::lir::store::expr_old::ops::ExprOpError;
use crate::aiplan4rust::lir::store::expr_old::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::tree::NodeId;

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
/// - This function is part of the **logic pipeline**, orchestrated by the
///   `normalize` module. Users should not call this directly unless implementing
///   a custom logic sequence.
///
/// # Parameters
/// - `root_id`: NodeId of the root of the subtree to process.
/// - `logic`: Mutable reference to the expression tree.
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
/// // Part of the logic pipeline managed by the `normalize` module
/// push_negation(root_id, &mut logic)?;
/// ```
pub fn push_negation(root_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;
        let kind = node.kind();

        if kind == ExprKind::Not {
            let child_id = node.children()[0];
            let child_kind = expr.try_node(child_id)?.kind();

            match child_kind {
                // (not (and A B)) -> (or (not A) (not B))
                ExprKind::And | ExprKind::Or => {
                    apply_de_morgan(node_id, expr)?;
                    stack.push(node_id);
                }

                // (not (forall (x) P)) -> (exists (x) (not P))
                ExprKind::Forall | ExprKind::Exists => {
                    apply_quantifier_negation(node_id, expr)?;
                    stack.push(node_id);
                }

                // --- OPTION A : TRAVERSÉE SANS MUTATION ---
                // On ne simplifie pas ¬¬X ici, on se contente de passer à travers
                // pour traiter les négations potentielles plus profondément.
                ExprKind::Not => {
                    let grandchild_id = expr.try_node(child_id)?.children()[0];
                    stack.push(grandchild_id);
                }

                // Négation sur une feuille (Atome/Comparaison) : On a atteint la cible.
                ExprKind::AtomicFormula | ExprKind::Comparison => continue,

                _ => return Err(ExprOpError::invalid_expr_node(child_id, child_kind)),
            }
        } else {
            // Exploration récursive standard
            let children = expr.try_node(node_id)?.children();
            for i in (0..children.len()).rev() {
                stack.push(children[i]);
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
/// - `node_id`: NodeId of the `Not` node to simplification.
/// - `logic`: Mutable reference to the expression tree containing the node.
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
fn apply_de_morgan(node_id: NodeId, expr: &mut Expr) -> Result<Vec<NodeId>, ExprOpError> {
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
    node_mut.set_kind(if child_kind == ExprKind::And {
        ExprKind::Or
    } else {
        ExprKind::And
    });
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
/// or logic later.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the `Not` node in the expression tree.
/// - `logic`: Mutable reference to the expression tree.
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
fn apply_quantifier_negation(node_id: NodeId, expr: &mut Expr) -> Result<NodeId, ExprOpError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::Not, "Node must be a Not");

    let child_id = node.children()[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(
        child.kind() == ExprKind::Forall || child.kind() == ExprKind::Exists,
        "Child of Not must be a quantifier"
    );
    debug_assert!(
        child.children().len() == 1,
        "Quantifier node must have exactly one child (the body)"
    );

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
    use crate::aiplan4rust::lir::store::expr_old::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::expr_old::ExprKind;

    /// Test pushing negation through AND using De Morgan's law.
    /// Input: (not (and (A) (B))) -> (or (not (A)) (not (B)))
    #[test]
    fn test_push_negation_and() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬(A ∧ B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let and_node = builder.and(vec![a, b]);
        let root = builder.not(and_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: De Morgan's Law ¬(A ∧ B) -> (¬A ∨ ¬B)
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 2);

        // Verify that all children are NOT nodes
        for &child_id in root.children() {
            assert_eq!(expr.try_node_kind(child_id)?, ExprKind::Not);

            // Bonus: verify that the leaf is an AtomicFormula
            let leaf_id = expr.try_node(child_id)?.children()[0];
            assert_eq!(expr.try_node_kind(leaf_id)?, ExprKind::AtomicFormula);
        }

        Ok(())
    }

    /// Test pushing negation through OR using De Morgan's law.
    /// Input: (not (or (A) (B))) -> (and (not (A)) (not (B)))
    #[test]
    fn test_push_negation_or() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬(A ∨ B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let or_node = builder.or(vec![a, b]);
        let root = builder.not(or_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: De Morgan's Law ¬(A ∨ B) -> (¬A ∧ ¬B)
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 2);

        // Verify that all children are NOT nodes
        for &child_id in root.children() {
            assert_eq!(expr.try_node_kind(child_id)?, ExprKind::Not);

            // Verify leaf is the expected AtomicFormula
            let leaf_id = expr.try_node(child_id)?.children()[0];
            assert_eq!(expr.try_node_kind(leaf_id)?, ExprKind::AtomicFormula);
        }

        Ok(())
    }

    /// Test pushing negation through a Forall quantifier.
    /// Input: (not (forall x (A))) -> (exists x (not (A)))
    #[test]
    fn test_push_negation_forall() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬(forall (?X - T) (A))
        let a = builder.atomic_formula(1, vec![]);
        let var_x = builder.typed_variable(10, &[100]); // ID 10, Type 100
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_node = builder.forall(forall_vars, a);
        let root = builder.not(forall_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: ¬∀x.A -> ∃x.¬A
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Exists);
        assert_eq!(root.children().len(), 1);

        // Verify the body of the Exists: (not (A))
        let body_id = root.children()[0];
        assert_eq!(expr.try_node_kind(body_id)?, ExprKind::Not);

        let inner_atom_id = expr.try_node(body_id)?.children()[0];
        assert_eq!(expr.try_node_kind(inner_atom_id)?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test pushing negation through an Exists quantifier.
    /// Input: (not (exists x (A))) -> (forall x (not (A)))
    #[test]
    fn test_push_negation_exists() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬(exists (?X - T) (A))
        let a = builder.atomic_formula(1, vec![]);
        let var_x = builder.typed_variable(10, &[100]); // ID 10, Type 100
        let exists_vars = builder.typed_variable_list(vec![var_x]);
        let exists_node = builder.exists(exists_vars, a);
        let root = builder.not(exists_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: ¬∃x.A -> ∀x.¬A
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Forall);
        assert_eq!(root.children().len(), 1);

        // Verify the body of the Forall: (not (A))
        let body_id = root.children()[0];
        assert_eq!(expr.try_node_kind(body_id)?, ExprKind::Not);

        let inner_atom_id = expr.try_node(body_id)?.children()[0];
        assert_eq!(expr.try_node_kind(inner_atom_id)?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test that no transformation occurs for a NOT whose child is neither AND/OR nor quantifier.
    /// Input: (not (A)) -> unchanged
    #[test]
    fn test_push_negation_no_change() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬A (déjà sous forme normale négative)
        let a = builder.atomic_formula(1, vec![]);
        let root = builder.not(a);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Aucun changement attendu
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Not);

        // Vérification de l'enfant unique
        let child_id = root.children()[0];
        assert_eq!(expr.try_node_kind(child_id)?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test pushing negation through a nested expression with AND, NOT, and quantifiers.
    /// Input: (not (and (A) (not (B)) (exists (?X) (C))))
    /// Expected output from push_negations alone:
    /// (or (not (A)) (not (not (B))) (forall (?X) (not (C))))
    #[test]
    fn test_push_negation_nested() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: ¬(A ∧ ¬B ∧ ∃x.C)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        let not_b = builder.not(b);
        let var_x = builder.typed_variable(10, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_x]);
        let exists_c = builder.exists(exists_vars, c);

        let and_node = builder.and(vec![a, not_b, exists_c]);
        let root = builder.not(and_node);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: ¬(A ∧ ¬B ∧ ∃x.C) -> (¬A ∨ ¬¬B ∨ ∀x.¬C)
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 3);

        // Premier fils : ¬A
        assert_eq!(expr.try_node_kind(root.children()[0])?, ExprKind::Not);

        // Deuxième fils : ¬¬B
        let second_id = root.children()[1];
        assert_eq!(expr.try_node_kind(second_id)?, ExprKind::Not);
        let inner_not_id = expr.try_node(second_id)?.children()[0];
        assert_eq!(expr.try_node_kind(inner_not_id)?, ExprKind::Not);

        // Troisième fils : ∀x.¬C
        let third_id = root.children()[2];
        assert_eq!(expr.try_node_kind(third_id)?, ExprKind::Forall);
        let body_id = expr.try_node(third_id)?.children()[0];
        assert_eq!(expr.try_node_kind(body_id)?, ExprKind::Not);

        Ok(())
    }

    #[test]
    fn test_push_negation_deep_nested() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup : ¬(A ∧ ¬(B ∨ C) ∧ ∀x.∃y.D)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let d = builder.atomic_formula(4, vec![]);

        let or_bc = builder.or(vec![b, c]);
        let not_or_bc = builder.not(or_bc);

        let var_y = builder.typed_variable(11, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_y]);
        let exists_d = builder.exists(exists_vars, d);
        let var_x = builder.typed_variable(10, &[100]);
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_exists_d = builder.forall(forall_vars, exists_d);

        let and_node = builder.and(vec![a, not_or_bc, forall_exists_d]);
        let root_id = builder.not(and_node);

        builder.set_root(root_id)?;
        let mut expr = builder.finish();

        // 2. Transformation : Résultat attendu -> (¬A ∨ ¬¬(B ∨ C) ∨ ∃x.∀y.¬D)
        // Note : On ne simplifie pas les doubles NOT ici (Option A). C'est le role de simplify
        push_negation(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(
            root.kind(),
            ExprKind::Or,
            "La racine doit être un OR après De Morgan"
        );
        assert_eq!(root.children().len(), 3);

        // --- Branche 0 : ¬A ---
        let branch_0_id = root.children()[0];
        assert_eq!(expr.try_node_kind(branch_0_id)?, ExprKind::Not);

        // --- Branche 1 : ¬¬(B ∨ C) ---
        // Puisqu'on ne fait que traverser, les deux NOT sont toujours présents.
        let first_not_id = root.children()[1];
        assert_eq!(
            expr.try_node_kind(first_not_id)?,
            ExprKind::Not,
            "Le premier NOT est conservé"
        );

        let second_not_id = expr.try_node(first_not_id)?.children()[0];
        assert_eq!(
            expr.try_node_kind(second_not_id)?,
            ExprKind::Not,
            "Le deuxième NOT est conservé"
        );

        let inner_or_id = expr.try_node(second_not_id)?.children()[0];
        assert_eq!(
            expr.try_node_kind(inner_or_id)?,
            ExprKind::Or,
            "On retrouve le OR initial"
        );

        // --- Branche 2 : ∃x.∀y.¬D ---
        let exists_id = root.children()[2];
        assert_eq!(expr.try_node_kind(exists_id)?, ExprKind::Exists);

        let forall_id = expr.try_node(exists_id)?.children()[0];
        assert_eq!(expr.try_node_kind(forall_id)?, ExprKind::Forall);

        let final_not_id = expr.try_node(forall_id)?.children()[0];
        assert_eq!(
            expr.try_node_kind(final_not_id)?,
            ExprKind::Not,
            "Le NOT a bien été poussé sur D"
        );

        let atom_d_id = expr.try_node(final_not_id)?.children()[0];
        assert_eq!(expr.try_node_kind(atom_d_id)?, ExprKind::AtomicFormula);

        Ok(())
    }
}
