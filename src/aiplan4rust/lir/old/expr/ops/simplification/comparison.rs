use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lir::old::expr::content::Content;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::{Expr, ExprKind};
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
/// * `logic` - The expression tree containing the node.
///
/// # Returns
///
/// * `Err(ExprError)` on structural access issues.
pub fn simplify(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::Comparison {
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
fn normalize_comparison(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;

    // We only handle FComp nodes
    if node.kind() != ExprKind::Comparison {
        return Ok(false);
    }

    let op = match node.content().as_compare_op() {
        Some(op) => op,
        None => return Ok(false), // should not happen normally
    };

    // Only > and >= need logic
    let new_op = match op {
        CompareOp::Greater => Some(CompareOp::Less),
        CompareOp::GreaterEq => Some(CompareOp::LessEq),
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
    node_mut.set_content(Content::Comparison(new_op.unwrap()));
    node_mut.set_children(vec![right, left]); // swap operands

    Ok(true)
}

/// Canonicalize commutative equality (=):
///   (= b a) → (= a b)
///
/// Returns Ok(true) if reordered.
fn canonicalize_comparison(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::Comparison {
        return Ok(false);
    }

    let op = match node.content().as_compare_op() {
        Some(op) => op,
        None => return Ok(false),
    };

    // Only = is commutative
    if op != CompareOp::Equal {
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
/// * `node_id` - The ID of the comparison node to simplification.
/// * `logic` - A mutable reference to the expression tree (`Expr`) containing the node.
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
fn simplify_comparison_constants(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let node = expr.try_node(node_id)?;

    // Ensure the node is of typing FComp
    if node.kind() != ExprKind::Comparison {
        return Ok(false);
    }

    let op = match node.content().as_compare_op() {
        Some(op) => op,
        None => {
            debug_assert!(false, "FComp node without BinaryComp content");
            return Ok(false); // should not happen in normal usage
        }
    };

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(
            children.len() == 2,
            "FComp node does not have exactly 2 children"
        );
        return Ok(false);
    }

    let left_id = children[0];
    let right_id = children[1];

    let left_node = expr.try_node(left_id)?;
    let right_node = expr.try_node(right_id)?;

    // 1. Tenter l'évaluation sur des nombres (Flottants / Ints)
    if let (Some(left_val), Some(right_val)) = (
        left_node.content().as_number(),
        right_node.content().as_number(),
    ) {
        let result = match op {
            CompareOp::Equal => left_val == right_val,
            CompareOp::Greater => left_val > right_val,
            CompareOp::Less => left_val < right_val,
            CompareOp::GreaterEq => left_val >= right_val,
            CompareOp::LessEq => left_val <= right_val,
        };
        // Replace the node using set_to_bool
        expr.set_to_bool(node_id, result)?;
        return Ok(true);
    }

    // 2. Tenter l'évaluation sur des objets (ExprKind::Constant)
    if left_node.kind() == ExprKind::Object && right_node.kind() == ExprKind::Object {
        // La seule comparaison valide sur des objets est l'égalité
        if op == CompareOp::Equal {
            // Si les sous-expressions sont identiques structurellement, c'est vrai, sinon faux.
            let is_eq = expr.deep_sub_expr_eq(left_id, right_id)?;
            expr.set_to_bool(node_id, is_eq)?;
            return Ok(true);
        }
    }

    Ok(false)
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
/// * `node_id` - The ID of the FComp node to simplification.
/// * `logic` - Mutable reference to the expression tree containing the node.
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
/// * `node_id` - The ID of the FComp node to simplification.
/// * `logic` - Mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was simplified.
/// * `Ok(false)` if no simplification was possible (different terms, not FComp, etc.).
/// * `Err(ExprError)` if accessing or mutating the node fails.
fn simplify_comparison_trivial_identity(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
    // 1. On extrait d'abord toutes les informations nécessaires du nœud parent
    let (left_id, right_id, op) = {
        let node = expr.try_node(node_id)?;

        if node.kind() != ExprKind::Comparison {
            return Ok(false);
        }

        let children = node.children();
        if children.len() != 2 {
            debug_assert!(
                children.len() == 2,
                "FComp node does not have exactly 2 children"
            );
            return Ok(false);
        }

        // On copie les IDs et on récupère l'opérateur de comparaison
        (children[0], children[1], node.content().as_compare_op())
    }; // L'emprunt immuable de 'node' s'arrête ICI.

    // 2. Maintenant expr est libre pour un emprunt mutable
    if expr.deep_sub_expr_eq(left_id, right_id)? {
        let is_true = matches!(
            op,
            Some(CompareOp::Equal) | Some(CompareOp::GreaterEq) | Some(CompareOp::LessEq)
        );

        // set_to_bool lèvera le flag hash_dirty
        expr.set_to_bool(node_id, is_true)?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{FunctionSymbolId, ObjectId, VariableId};
    use crate::aiplan4rust::lir::old::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::old::expr::ExprKind;

    /// Test que l'égalité entre deux objets identiques est simplifiée en `and` (True).
    /// Entrée : (= a a) où 'a' est une constante symbolique (objet).
    /// Attendu : (and)
    #[test]
    fn test_simplify_object_constant_equal() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= a a)
        // On crée deux nœuds distincts, mais représentant le même objet "a"
        let a = ObjectId::from(1);
        let left = builder.constant(a);
        let right = builder.constant(a);
        let eq_node = builder.equal(left, right);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= "a" "a") -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // L'égalité disparait au profit d'un (and) vide (True)
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "L'égalité d'objets identiques doit devenir un 'and' vide"
        );
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test que l'égalité entre deux constantes d'objets différents est simplifiée en `or` (False).
    /// Entrée : (= c1 c2)
    /// Attendu : (or)
    #[test]
    fn test_simplify_object_constant_not_equal() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= obj_1 obj_2)
        let left = builder.constant(ObjectId::from(1));
        let right = builder.constant(ObjectId::from(2));
        let eq_node = builder.equal(left, right);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // L'égalité de valeurs différentes devient False (or vide)
        assert_eq!(
            root_node.kind(),
            ExprKind::Or,
            "L'égalité d'objets différents doit être simplifiée en False"
        );
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test que l'égalité entre une variable et elle-même est simplifiée en `and` (True).
    /// Entrée : (= ?v1 ?v1)
    /// Attendu : (and)
    #[test]
    fn test_simplify_variable_self_equal() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= ?var_x ?var_x)
        // On utilise le même VariableId pour simuler la même variable
        let var_id = VariableId::from(42);
        let left = builder.variable(var_id);
        let right = builder.variable(var_id);
        let eq_node = builder.equal(left, right);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: Simplify (= ?x ?x) -> True
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // Une variable est toujours égale à elle-même
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "L'égalité d'une variable avec elle-même doit devenir un 'and' vide (True)"
        );
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test que l'égalité entre deux variables différentes n'est PAS simplifiée.
    /// Entrée : (= ?v1 ?v2)
    /// Attendu : (= ?v1 ?v2) (inchangé car elles pourraient être égales ou non à l'exécution)
    #[test]
    fn test_simplify_different_variables_no_change() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (= ?var_1 ?var_2)
        let v1 = builder.variable(VariableId::from(1));
        let v2 = builder.variable(VariableId::from(2));
        let eq_node = builder.equal(v1, v2);

        builder.set_root(eq_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation
        simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // On ne peut pas simplifier car on ne connaît pas les valeurs futures des variables
        assert_eq!(
            root_node.kind(),
            ExprKind::Comparison,
            "L'égalité entre deux variables distinctes ne doit pas être simplifiée"
        );
        // Vérification du contenu (l'opérateur doit être Equal)
        assert_eq!(
            root_node.content().try_compare_op()?,
            CompareOp::Equal,
            "L'opérateur de comparaison doit toujours être 'Equal'"
        );
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }
    /// Teste si le moteur de simplification reconnaît que f(?x, ?y) = f(?x, ?y) est vrai.
    #[test]
    fn test_simplify_function_equality() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Définition des IDs pour les variables et la fonction
        let var_x_id = VariableId::from(1);
        let var_y_id = VariableId::from(2);
        let func_symbol_id = FunctionSymbolId::from(10);

        // 2. Construction du premier terme : f(?x, ?y)
        let x1 = builder.variable(var_x_id);
        let y1 = builder.variable(var_y_id);
        let f1 = builder.function_term(func_symbol_id, vec![x1, y1]);

        // 3. Construction du second terme : f(?x, ?y)
        let x2 = builder.variable(var_x_id);
        let y2 = builder.variable(var_y_id);
        let f2 = builder.function_term(func_symbol_id, vec![x2, y2]);

        // 4. Création du prédicat d'égalité : f(?x, ?y) == f(?x, ?y)
        let equality_node = builder.comparison(CompareOp::Equal, f1, f2);

        // Définition de la racine de l'expression
        builder.set_root(equality_node)?;
        let mut expr = builder.finish();

        // 5. Exécution de la simplification
        // La méthode simplification() doit réduire l'égalité de deux termes identiques à "True"
        simplify(equality_node, &mut expr)?;

        // 6. Vérification du résultat
        let root_node = expr.try_root_node()?;

        // Une égalité tautologique doit devenir un 'and' vide (représentant 'True')
        assert!(
            root_node.is_empty_and(),
            "L'égalité d'une expression avec elle-même doit devenir un 'and' vide (True)"
        );
        Ok(())
    }

    /// Test that a constant equality comparison is simplified to `and`.
    /// Input: (= 3 3)
    /// Expected: (and)
    #[test]
    fn test_simplify_constant_equal() -> Result<(), ExprOpError> {
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
    fn test_simplify_constant_not_equal() -> Result<(), ExprOpError> {
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
    fn test_simplify_identical_variables() -> Result<(), ExprOpError> {
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
    fn test_simplify_structural_identity() -> Result<(), ExprOpError> {
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
    fn test_no_simplification_different_variables() -> Result<(), ExprOpError> {
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
        assert_eq!(root_node.kind(), ExprKind::Comparison);
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }

    /// Test that greater-equal on identical variables simplifies to `and`.
    /// Input: (>= ?x ?x)
    /// Expected: (and)
    #[test]
    fn test_simplify_greater_eq_identity() -> Result<(), ExprOpError> {
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
    fn test_simplify_greater_identity() -> Result<(), ExprOpError> {
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
