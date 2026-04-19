use crate::aiplan4rust::lir::expr::ops::simplification::{
    and_or, arithmetic, assign, comparison, not, quantifier, when,
};
use crate::aiplan4rust::lir::expr::ops::{ExprOpError, StaticEvaluator, StaticValue};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::tree::NodeId;

/// Simplifies a PDDL-like expression tree in a post-order traversal.
///
/// This function performs a **full simplification pass** over the given expression tree.
/// It traverses the tree in **post-order** (children before parent) and applies
/// node-specific simplification functions (`simplify_node`) to each node.
///
/// # Preconditions
/// For the simplification to work correctly, the expression tree must satisfy the following:
/// 1. **All `Imply` nodes have been removed** or transformed (e.g., `Imply(A, B) -> Or(Not(A), B)`).
///    `Imply` nodes are not handled in `simplify_node`.
/// 2. **Negations have been pushed down** to atomic formulas using De Morgan's laws and quantifier rules:
///    - `Not(And(A, B)) -> Or(Not(A), Not(B))`
///    - `Not(Or(A, B)) -> And(Not(A), Not(B))`
///    - `Not(Forall(vars, body)) -> Exists(vars, Not(body))`
///    - `Not(Exists(vars, body)) -> Forall(vars, Not(body))`
/// 3. **Temporal specifiers (`AtStart`, `AtEnd`, `Overall`) have been factorized** so that each modifier
///    appears only once at the top of conjunctions. For example:
///        ```lisp
///        (and (at-start (A)) (at-start (B)) (at-end (C)))
///        => (and (at-start (A) (B)) (at-end (C)))
///        ```
///
/// # Parameters
/// - `root_id`: The ID of the root node of the expression tree.
/// - `logic`: A mutable reference to the expression tree (`Expr`) to be simplified.
///
/// # Behavior
/// 1. Performs a **depth-first search (DFS)** in post-order using an explicit stack to avoid recursion:
///     - Each stack entry is `(node_id, visited)` where `visited` indicates if children have already been processed.
///     - Children are pushed first, then the parent is revisited to ensure post-order processing.
/// 2. After constructing the post-order list of node IDs, each node is simplified by calling `simplify_node(node_id, logic)`.
/// 3. Node-specific simplifications include:
///     - `And` / `Or`: flattening, deduplication, reducing single-child nodes.
///     - `Not`: already pushed down; can be further simplified if nested.
///     - `Forall` / `Exists`: body simplification and variable factoring.
///     - `When`: simplification conditional effects.
///     - Temporal nodes (`AtStart`, `AtEnd`, `Overall`): already factorized, nothing more to do.
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully.
/// - `Err(ExprError)` if any node access or mutation fails during traversal or simplification.
///
/// # Notes
/// - Post-order traversal ensures that children are simplified before their parents,
///   which is essential for flattening, deduplication, and factorization.
/// - Nodes that do not require simplification (types, constants, variables, etc.) are skipped.
///
/// # Example
/// ```ignore
/// let mut logic = build_expr_tree();
/// let root_id = logic.root_id().unwrap();
///
/// // Preconditions must be satisfied before calling simplification:
/// // - All Implies removed
/// // - Negations pushed down
/// // - Temporal specifiers factorized
///
/// simplification(root_id, &mut logic)?;
/// ```
pub fn simplify_with(
    expr: &mut Expr,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), ExprOpError> {
    if let Some(root_id) = expr.root_id() {
        simplify_subexpr_with(expr, root_id, evaluator)?;
    }
    Ok(())
}

pub fn simplify(expr: &mut Expr) -> Result<(), ExprOpError> {
    if let Some(root_id) = expr.root_id() {
        simplify_subexpr_with(expr, root_id, None)?;
    }
    Ok(())
}

pub fn simplify_subexpr(expr: &mut Expr, node_id: NodeId) -> Result<(), ExprOpError> {
    if let Some(root_id) = expr.root_id() {
        simplify_subexpr_with(expr, node_id, None)?;
    }
    Ok(())
}

/// Réduit uniquement un sous-arbre à partir d'un noeud spécifique.
/// C'est cette variante que tu appelles dans ta boucle d'expansion.
/*pub fn simplify_subexpr_with(
    expr: &mut Expr,
    node_id: NodeId,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), ExprOpError> {
    let has_eval = evaluator.is_some();

    // 1. Collecte et filtrage (O(N))
    let ids: Vec<NodeId> = expr
        .postorder_from(node_id)
        .ids()
        .filter(|(_, node)| is_simplifiable(node.kind(), has_eval))
        .map(|(id, _)| id)
        .collect();

    // 2. Transformation directe (O(K))
    // On traite chaque nœud. Pas de check has_node car la simplification est locale.
    for id in ids {
        simplify_node(id, expr, evaluator)?;
    }

    Ok(())
}*/

/*pub fn simplify_subexpr_with(
    expr: &mut Expr,
    node_id: NodeId,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), ExprOpError> {
    let has_eval = evaluator.is_some();

    // 1. Collecte initiale (Post-order)
    let ids: Vec<NodeId> = expr
        .postorder_from(node_id)
        .ids()
        .map(|(id, _)| id)
        .collect();

    // 2. Traitement itératif
    for id in ids {
        // IMPORTANT : On vérifie si le nœud existe toujours avant de faire quoi que ce soit
        let node_ref = match expr.get_node(id) {
            Some(n) => n,
            None => {
                // Le nœud a été supprimé par la simplification d'un de ses ancêtres/frères
                continue;
            }
        };

        let kind_before = node_ref.kind();

        // On ne simplifie que si nécessaire
        if is_simplifiable(kind_before, has_eval) {
            // --- CAPTURE DE L'ÉTAT RÉEL DU NŒUD COURANT (id, pas node_id) ---
            let children_before = node_ref.children().to_vec();

            // On lance la simplification
            // Idéalement, simplify_node devrait retourner Ok(bool) pour savoir si ça a bougé
            simplify_node(id, expr, evaluator)?;

            // --- VÉRIFICATION DE L'ÉTAT APRÈS ---
            match expr.get_node(id) {
                Some(new_node) => {
                    let kind_after = new_node.kind();
                    let children_after = new_node.children();

                    // On logue SI et seulement SI le nœud lui-même a changé
                    if kind_after != kind_before || children_after != children_before {
                        println!(
                            "[SIMPLIFY-TRACE] Node #{}: {}({:?}) -> {}({:?})",
                            id, kind_before, children_before, kind_after, children_after
                        );
                    }
                }
                None => {
                    println!(
                        "[SIMPLIFY-TRACE] Node #{}: {} a été SUPPRIMÉ ou fusionné",
                        id, kind_before
                    );
                }
            }
        }
    }

    Ok(())
}*/

enum Step {
    Discover(NodeId),
    Process(NodeId),
}

pub fn simplify_subexpr_with(
    expr: &mut Expr,
    node_id: NodeId,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), ExprOpError> {
    let has_eval = evaluator.is_some();

    // Notre pile remplace l'itérateur statique
    let mut stack = vec![Step::Discover(node_id)];

    while let Some(step) = stack.pop() {
        match step {
            Step::Discover(id) => {
                // On prévoit de traiter ce nœud APRES ses enfants (Post-order)
                stack.push(Step::Process(id));

                // On empile les enfants pour les découvrir
                // On récupère les IDs via un emprunt court pour ne pas bloquer expr
                if let Some(node) = expr.get_node(id) {
                    // On empile à l'envers pour maintenir l'ordre visuel (facultatif)
                    for &child_id in node.children().iter().rev() {
                        stack.push(Step::Discover(child_id));
                    }
                }
            }
            Step::Process(id) => {
                // --- C'EST ICI QUE LA MAGIE OPERE ---
                // On vérifie l'état actuel du nœud dans l'Arena
                let node_ref = match expr.get_node(id) {
                    Some(n) => n,
                    None => continue, // Le nœud a disparu (ex: absorbé par un move_to précédent)
                };

                let kind_before = node_ref.kind();

                if is_simplifiable(kind_before, has_eval) {
                    let children_before = node_ref.children().to_vec();

                    // Mutation de l'expr (le Borrow Checker est ok car la stack ne contient que des IDs/Enums)
                    simplify_node(id, expr, evaluator)?;

                    // Trace optionnelle pour débugger ton domaine
                    if let Some(new_node) = expr.get_node(id) {
                        let kind_after = new_node.kind();
                        let children_after = new_node.children();

                        if kind_after != kind_before || children_after != children_before {
                            println!(
                                "[SIMPLIFY-TRACE] Node #{}: {}({:?}) -> {}({:?})",
                                id, kind_before, children_before, kind_after, children_after
                            );
                        }
                    } else {
                        println!(
                            "[SIMPLIFY-TRACE] Node #{}: {} a été ABSORBÉ",
                            id, kind_before
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Détermine si un nœud nécessite un traitement par `simplify_node`.
/// Cette fonction est interne au module de simplification.
/// Détermine si un nœud mérite d'être visité par le moteur de simplification.
/// On filtre ici pour ne pas charger le Vec d'IDs avec des feuilles inertes.
fn is_simplifiable(kind: ExprKind, has_evaluator: bool) -> bool {
    match kind {
        // Nœuds avec une logique de réduction active
        ExprKind::And
        | ExprKind::Or
        | ExprKind::Not
        | ExprKind::Forall
        | ExprKind::Exists
        | ExprKind::When
        | ExprKind::Assignment
        | ExprKind::Comparison
        | ExprKind::Arithmetic
        | ExprKind::Imply => true, // Most

        // Atomes (Prédicats/Fonctions) : seulement si on a un évaluateur
        ExprKind::AtomicFormula | ExprKind::Function => has_evaluator,

        // Feuilles inertes (Number, Constant, Variable, etc.)
        _ => false,
    }
}

/// Simplifies a node in a PDDL expression tree based on its kind.
///
/// This function inspects the typing of the node identified by `node_id` and applies
/// the appropriate simplification routine for that typing. Currently, it only handles
/// `AND` and `OR` nodes by delegating to `simplify_and_or_node`.
/// Nodes of other kinds are left unchanged.
///
/// # Parameters
/// - `node_id`: The ID of the node to simplification.
/// - `logic`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the simplification succeeds or the node typing is not handled.
/// - `Err(ExprError)` if accessing the node fails.
///
/// # Notes
/// - This function is intended to be called from a post-order traversal of the
///   expression tree, so that children are simplified before their parent.
/// - Extending this function to support additional node kinds (e.g., `NOT`,
///   arithmetic logic) is straightforward: simply add a match arm
///   for the new kind.
///
/// # Example
/// ```ignore
/// let node_id = logic.root_id().unwrap();
/// simplify_node(node_id, &mut logic)?;
/// ```
fn simplify_node(
    node_id: NodeId,
    expr: &mut Expr,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), ExprOpError> {
    let kind = expr.try_node(node_id)?.kind();

    match kind {
        // Opérateurs logiques et arithmétiques (inchangé)
        ExprKind::And | ExprKind::Or => and_or::simplify(node_id, expr)?,
        ExprKind::Not => not::simplify(node_id, expr)?,
        ExprKind::Forall | ExprKind::Exists => quantifier::simplify(node_id, expr)?,
        ExprKind::Assignment => assign::simplify(node_id, expr)?,
        ExprKind::Comparison => comparison::simplify(node_id, expr)?,
        ExprKind::Arithmetic => arithmetic::simplify(node_id, expr)?,
        ExprKind::When => when::simplify(node_id, expr)?,

        ExprKind::Imply => return Err(ExprOpError::invalid_expr_node(node_id, ExprKind::Imply)),

        // Unification du traitement AtomicFormula (Prédicats) et FunctionTerm
        ExprKind::AtomicFormula | ExprKind::Function => {
            if let Some(eval) = evaluator {
                // On utilise la méthode unique du trait
                if let Some(static_val) = eval.evaluate(node_id, expr) {
                    println!("[INERTIA-TRACE] Node {} évalué à {:?}", node_id, static_val);
                    match static_val {
                        StaticValue::Boolean(is_true) => {
                            expr.set_to_bool(node_id, is_true)?;
                        }
                        StaticValue::Number(num) => {
                            expr.set_to_number(node_id, num)?;
                        }
                        StaticValue::Object(obj) => {
                            expr.set_to_object(node_id, obj)?;
                        }
                    }
                }
            }
        }

        _ => {}
    }

    Ok(())
}
