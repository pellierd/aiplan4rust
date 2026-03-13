use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::{NodeId, Node};

/// Simplifies a quantifier node (`forall` or `exists`) by applying a sequence of transformations.
///
/// This function applies the following steps in order:
/// 1. Canonicalizes the variables in the `TypedList` of the quantifier.
/// 2. Removes the quantifier if the variable list is empty, replacing it with its body.
/// 3. Fuses nested quantifiers of the same kind, combining their variable lists.
/// 4. Simplifies the quantifier if its body is trivially true or false (e.g., `(forall (x) (and))` → `(and)`).
///
/// Each step is applied only if applicable. The first step that modifies the expression may
/// terminate the expr early if the node is replaced or simplified.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the quantifier node to normalize.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if expr completes successfully (even if no changes were made).
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
pub fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprOpError> {
    // Étape 1 : Mise en forme canonique (tri des variables)
    canonicalize_quantifier_vars(node_id, expr)?;

    // Étape 2 : Nettoyage structurel (si aucune variable, on remonte le corps)
    // (forall () Body) -> Body
    if remove_empty_quantifier(node_id, expr)? {
        return Ok(());
    }

    // Étape 3 : Fusion (forall (x) (forall (y) B)) -> (forall (x y) B)
    fuse_nested_quantifiers(node_id, expr)?;

    // Étape 4 : Constantes triviales (corps déjà réduit à True/False)
    // (forall (x) true) -> true
    if simplify_quantifier_trivial_body(node_id, expr)? {
        return Ok(());
    }

    // Étape 5 : Vers l'Elimination (Le futur)
    // Ici, on pourrait ajouter l'élimination des variables si le domaine
    // d'un either_type est connu et statique.

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
pub fn canonicalize_quantifier_vars(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    // 1. Vérification rapide en immuable
    {
        let node = expr.try_node(node_id)?;

        debug_assert!(
            node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
            "Node must be a quantifier (Forall or Exists)"
        );

        let vars = node.content().try_quantifier_vars()?;
        if vars.len() <= 1 {
            return Ok(()); // Pas besoin de trier 0 ou 1 variable
        }
    }

    // 2. Action : Tri des variables
    let node_mut = expr.try_node_mut(node_id)?;
    let vars = node_mut.content_mut().try_quantifier_vars_mut()?;

    // On trie par symbole pour garantir que (forall (?a ?b) ...)
    // soit identique à (forall (?b ?a) ...) après simplification.
    vars.sort_by_symbol();

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
/// - `node_id`: The `NodeId` of the quantifier node (`Forall` or `Exists`) to simplification.
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
pub fn remove_empty_quantifier(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    // 1. Phase de lecture immuable (plus rapide et sécurisée pour les asserts)
    let (is_empty, body_id) = {
        let node = expr.try_node(node_id)?;

        debug_assert!(
            node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
            "Node must be a quantifier (Forall or Exists)"
        );
        debug_assert!(
            node.children().len() == 1,
            "Quantifier must have exactly 1 child"
        );

        let vars = node.content().try_quantifier_vars()?;
        (vars.is_empty(), node.children()[0])
    };

    // 2. Phase d'action : si aucune variable n'est quantifiée, le nœud est inutile
    if is_empty {
        // (forall () Body) -> Body
        // (exists () Body) -> Body
        expr.move_to(body_id, node_id)?;
        return Ok(true);
    }

    Ok(false)
}

/// Fuses immediate nested quantifiers of the same kind (`forall` or `exists`) into a single node.
///
/// This function merges a quantifier with its direct child quantifier of the same either_type.
/// The variables from the inner quantifier are moved into the outer quantifier's
/// `QuantifierVariables` content. The body of the inner quantifier replaces the
/// body of the outer quantifier.
///
/// Only immediate nested quantifiers of the same either_type are fused. If the outer node
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
pub fn fuse_nested_quantifiers(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    // Étape 1 : Lecture immuable
    let (outer_kind, inner_id) = {
        let outer_node = expr.try_node(node_id)?;
        let outer_kind = outer_node.kind();
        debug_assert!(
            outer_kind == ExprKind::Forall || outer_kind == ExprKind::Exists,
            "Outer node must be a quantifier (Forall or Exists)"
        );
        let outer_children = outer_node.children();
        debug_assert!(outer_children.len() == 1, "Outer quantifier must have exactly one child");
        (outer_kind, outer_children[0])
    };

    // Étape 2 : Vérification du nœud interne
    let inner_node = expr.try_node(inner_id)?;
    if inner_node.kind() != outer_kind {
        return Ok(()); // Kind différent, pas de fusion possible
    }
    debug_assert!(inner_node.children().len() == 1, "Inner quantifier must have exactly one child");
    let inner_body_id = inner_node.children()[0];

    // Étape 3 : Fusion des variables (Optimisé avec extend)
    let inner_vars = {
        let inner_node_mut = expr.try_node_mut(inner_id)?;
        std::mem::take(inner_node_mut.content_mut().try_quantifier_vars_mut()?)
    };

    {
        let outer_node_mut = expr.try_node_mut(node_id)?;
        let outer_vars = outer_node_mut.content_mut().try_quantifier_vars_mut()?;

        outer_vars.extend(inner_vars);
        outer_vars.sort_by_symbol();
        outer_vars.dedup_by_symbol();
    }

    // Étape 4 : Court-circuit (Le "Move" de structure)
    // Au lieu de copier le contenu du corps dans inner_id,
    // on dit simplement au parent (node_id) que son nouvel enfant est le petit-enfant.
    {
        let outer_node_mut = expr.try_node_mut(node_id)?;
        outer_node_mut.set_children(vec![inner_body_id]);
    }

    // Note : inner_id est maintenant orphelin, il sera ignoré par le reste du parcours.
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
/// - `node_id`: The `NodeId` of the quantifier node to simplification.
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
) -> Result<bool, ExprOpError> {
    // 1. On récupère l'ID de l'enfant et on valide le parent
    let body_id = {
        let node = expr.try_node(node_id)?;

        debug_assert!(
            node.kind() == ExprKind::Forall || node.kind() == ExprKind::Exists,
            "Node must be a quantifier (Forall or Exists)"
        );
        debug_assert!(
            node.children().len() == 1,
            "Quantifier must have exactly 1 child"
        );

        node.children()[0]
    };

    // 2. On vérifie si l'enfant est une constante vide (And/Or)
    let is_trivial = {
        let body = expr.try_node(body_id)?;

        // Un corps est trivial s'il n'a pas d'enfants et que c'est un connecteur logique
        body.children().is_empty() && (body.kind() == ExprKind::And || body.kind() == ExprKind::Or)
    };

    if is_trivial {
        // 3. LE MOVE_TO : On écrase le quantificateur par son corps constant.
        // On déplace directement le contenu de body_id dans node_id.
        expr.move_to(body_id, node_id)?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::TypedList;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::{ExprError, ExprKind};
    use crate::aiplan4rust::lir::expr::ops::ExprOpError;
    use crate::aiplan4rust::lir::expr::ops::simplification::quantifier;

    /// Test that an empty forall quantifier is replaced by its body.
    /// Input: (forall () (A))
    /// Expected: (A)
    #[test]
    fn test_remove_empty_forall() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (forall () (A))
        let atomic_a = builder.atomic_formula(1, vec![]); // "A"
        let forall_node = builder.forall(TypedList::new(), atomic_a);

        builder.set_root(forall_node)?;
        let mut expr = builder.finish();
        let root_id = expr.try_root_id()?;

        // 2. Transformation: simplification (quantifier::simplification)
        // Le quantificateur sans variables est élagué.
        quantifier::simplify(root_id, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_node(root_id)?;

        // Le nœud ForAll doit avoir disparu au profit de son enfant unique (A)
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }

       /// Test that nested forall quantifiers are fused.
       /// Input: (forall (?X - T2) (forall (?Y - T1) (A)))
       /// Expected: (forall (?X - T2 ?Y - T1) (A))
       #[test]
       fn test_fuse_nested_forall_structured() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables (Méthode recommandée)
           let var_y = builder.typed_variable(1, &[101]); // ?Y - T1
           let list_y = builder.typed_variable_list(vec![var_y]);

           let var_x = builder.typed_variable(2, &[102]); // ?X - T2
           let list_x = builder.typed_variable_list(vec![var_x]);

           // 2. Construction de l'arbre
           let atomic_a = builder.atomic_formula(3, vec![]);
           let inner_forall = builder.forall(list_y, atomic_a);
           let outer_forall = builder.forall(list_x, inner_forall);

           builder.set_root(outer_forall)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 3. Transformation
           quantifier::simplify(root_id, &mut expr)?;

           // 4. Validation
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::Forall);

           // On vérifie que la liste fusionnée contient bien les 2 variables
           let fused_vars = root_node.content().try_quantifier_vars()?;
           assert_eq!(fused_vars.len(), 2);

           Ok(())
       }

       /// Test that a quantifier with trivial body is replaced by its body.
       /// Input: (forall (?X - T) (and))
       /// Expected: (and)
       #[test]
       fn test_trivial_body_exists_false() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables (Méthode propre)
           let var_x = builder.typed_variable(10, &[100]);
           let var_list = builder.typed_variable_list(vec![var_x]);

           // 2. Le corps est une contradiction : (or)
           let empty_or = builder.or(vec![]);

           // 3. Création du exists
           let exists_node = builder.exists(var_list, empty_or);

           builder.set_root(exists_node)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 4. Simplification
           quantifier::simplify(root_id, &mut expr)?;

           // 5. Validation : (exists (?x) (or)) -> (or)
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::Or);
           assert!(root_node.children().is_empty());

           Ok(())
       }

       /// Test that no simplification is applied when quantifier is non-empty, non-nested, non-trivial.
       /// Input: (forall (?X - T) (A))
       /// Expected unchanged
       #[test]
       fn test_no_simplification_forall() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables (Ta méthode propre)
           let var_x = builder.typed_variable(10, &[100]);
           let var_list = builder.typed_variable_list(vec![var_x]);

           // 2. Le corps est un prédicat atomique (non simplifiable par défaut)
           let atomic_a = builder.atomic_formula(3, vec![]);

           // 3. Création du forall
           let forall_node = builder.forall(var_list, atomic_a);

           builder.set_root(forall_node)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 4. Simplification : ne devrait rien changer
           quantifier::simplify(root_id, &mut expr)?;

           // 5. Validation
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::Forall);

           Ok(())
       }

       /// Test that an empty exists quantifier is replaced by its body.
       /// Input: (exists () (A))
       /// Expected: (A)
       #[test]
       fn test_remove_empty_exists() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation d'une liste de variables vide
           let var_list = builder.typed_variable_list(vec![]);

           // 2. Construction : (exists () (A))
           let atomic_a = builder.atomic_formula(3, vec![]); // ID 3 pour "A"
           let exists_node = builder.exists(var_list, atomic_a);

           builder.set_root(exists_node)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 3. Transformation
           quantifier::simplify(root_id, &mut expr)?;

           // 4. Validation
           let root_node = expr.try_node(root_id)?;

           // Le quantificateur a été supprimé, la racine est maintenant l'AtomicFormula
           assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

           Ok(())
       }

       /// Test that nested exists quantifiers are fused into a single node.
       /// Input: (exists (?X - T2) (exists (?Y - T1) (A)))
       /// Expected: (exists (?Y - T1 ?X - T2) (A))
       #[test]
       fn test_fuse_nested_exists() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables séparément (Méthode recommandée)
           let var_y = builder.typed_variable(1, &[101]); // ?Y - T1
           let list_y = builder.typed_variable_list(vec![var_y]);

           let var_x = builder.typed_variable(2, &[102]); // ?X - T2
           let list_x = builder.typed_variable_list(vec![var_x]);

           // 2. Construction de l'arbre : (exists (?X) (exists (?Y) (A)))
           let atomic_a = builder.atomic_formula(3, vec![]);
           let inner_exists = builder.exists(list_y, atomic_a);
           let outer_exists = builder.exists(list_x, inner_exists);

           builder.set_root(outer_exists)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 3. Transformation : simplification
           quantifier::simplify(root_id, &mut expr)?;

           // 4. Validation
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::Exists);

           // On vérifie que les variables sont fusionnées (longueur 2)
           let fused_vars = root_node.content().try_quantifier_vars()?;
           assert_eq!(fused_vars.len(), 2);

           Ok(())
       }


       /// Test that an exists quantifier with a trivial body is replaced by its body.
       /// Input: (exists (?X - T) (and))
       /// Expected: (and)
       #[test]
       fn test_trivial_body_exists() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables (Méthode à plat)
           let var_x = builder.typed_variable(10, &[100]);
           let var_list = builder.typed_variable_list(vec![var_x]);

           // 2. Le corps est une tautologie : (and)
           let empty_and = builder.and(vec![]);

           // 3. Création du exists
           let exists_node = builder.exists(var_list, empty_and);

           builder.set_root(exists_node)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 4. Simplification
           quantifier::simplify(root_id, &mut expr)?;

           // 5. Validation : (exists (?x) (and)) -> (and)
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::And);
           assert!(root_node.children().is_empty());

           Ok(())
       }

       /// Test that no simplification is applied on a non-empty, non-nested exists quantifier.
       /// Input: (exists (?X - T) (A))
       /// Expected unchanged: (exists (?X - T) (A))
       #[test]
       fn test_no_simplification_exists() -> Result<(), ExprOpError> {
           let mut builder = ExprBuilder::new();

           // 1. Préparation des variables (Méthode à plat)
           let var_x = builder.typed_variable(10, &[100]);
           let var_list = builder.typed_variable_list(vec![var_x]);

           // 2. Le corps est un prédicat atomique : (A)
           let atomic_a = builder.atomic_formula(3, vec![]);

           // 3. Création du exists
           let exists_node = builder.exists(var_list, atomic_a);

           builder.set_root(exists_node)?;
           let mut expr = builder.finish();
           let root_id = expr.try_root_id()?;

           // 4. Simplification : Ne doit rien changer
           quantifier::simplify(root_id, &mut expr)?;

           // 5. Validation : (exists (?x) (A)) reste (exists (?x) (A))
           let root_node = expr.try_node(root_id)?;
           assert_eq!(root_node.kind(), ExprKind::Exists);

           Ok(())
       }
}
