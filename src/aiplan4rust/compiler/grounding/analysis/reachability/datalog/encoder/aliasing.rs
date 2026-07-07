use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::{CompareOp, VariableId};
use crate::analysis::reachability::datalog::core::Term;
use crate::analysis::reachability::datalog::error::DatalogError;

/// Version locale (associée) pour extraire la table des alias d'un groupe d'égalité
/// Version locale (associée) pour extraire la table des alias d'un groupe d'égalité
pub(crate) fn extract_variable_aliases(
    effect_id: ExprId,
    store: &ExprStore,
) -> Result<std::collections::HashMap<VariableId, Term>, DatalogError> {
    let mut aliases = std::collections::HashMap::new();

    // Gardien du DAG (Hash-Consing) - Allocation unique de la taille du store
    let mut visited = vec![false; store.len()];
    let mut stack = vec![effect_id];

    // 🌟 Version optimisée : Zéro allocation, parcours direct et sécurisé par le tri des IDs
    let find_rep = |map: &std::collections::HashMap<VariableId, Term>, v: VariableId| -> Term {
        let mut curr = Term::Variable(v);
        while let Term::Variable(var) = curr {
            if let Some(next) = map.get(&var) {
                curr = next.clone();
            } else {
                break;
            }
        }
        curr
    };

    while let Some(node_id) = stack.pop() {
        let idx = node_id.as_usize();
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        // 🌟 Appel direct au store au lieu de expr
        let node = store.fetch(node_id)?;
        let kind = node.kind();

        match kind {
            ExprKind::Not => {
                continue; // Les égalités dans un NOT sont des inégalités, on ignore.
            }

            ExprKind::Comparison(op) => {
                if *op == CompareOp::Equal {
                    let children = node.children();
                    if children.len() == 2 {
                        // 💡 Appels mis à jour pour passer l'ID et le store
                        let t1 = node_to_term(children[0], store)?;
                        let t2 = node_to_term(children[1], store)?;

                        match (t1, t2) {
                            (Some(Term::Variable(v1)), Some(Term::Variable(v2))) => {
                                let r1 = find_rep(&aliases, v1);
                                let r2 = find_rep(&aliases, v2);

                                if r1 != r2 {
                                    match (r1, r2) {
                                        (Term::Variable(var1), Term::Variable(var2)) => {
                                            aliases.insert(
                                                var1.max(var2),
                                                Term::Variable(var1.min(var2)),
                                            );
                                        }
                                        (Term::Variable(var), Term::Constant(c))
                                        | (Term::Constant(c), Term::Variable(var)) => {
                                            aliases.insert(var, Term::Constant(c));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            (Some(Term::Variable(v)), Some(Term::Constant(c)))
                            | (Some(Term::Constant(c)), Some(Term::Variable(v))) => {
                                let r = find_rep(&aliases, v);
                                if let Term::Variable(var) = r {
                                    aliases.insert(var, Term::Constant(c));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ => {
                for &child_id in node.children().iter().rev() {
                    stack.push(child_id);
                }
            }
        }
    }

    // Aplatissement final unique (Path Compression)
    compute_transitive_closure(&mut aliases);
    Ok(aliases)
}

/// Version locale (associée) pour convertir un identifiant de nœud en Term
fn node_to_term(node_id: ExprId, store: &ExprStore) -> Result<Option<Term>, DatalogError> {
    // 🌟 Appel direct au store pour récupérer le nœud
    let n = store.fetch(node_id)?;

    Ok(match n.kind() {
        ExprKind::Variable(v_id) => Some(Term::Variable(*v_id)),
        ExprKind::Object(obj_id) => Some(Term::Constant(*obj_id)),
        _ => None,
    })
}

/// Version locale (associée) pour calculer la fermeture transitive (Path Compression)
fn compute_transitive_closure(aliases: &mut std::collections::HashMap<VariableId, Term>) {
    let keys: Vec<VariableId> = aliases.keys().cloned().collect();

    for start_var in keys {
        // On récupère le terme cible initial
        let mut current_term = aliases.get(&start_var).unwrap().clone();
        let mut visited = std::collections::HashSet::new();
        visited.insert(start_var);

        // On suit la chaîne des variables aliasées
        while let Term::Variable(v) = current_term {
            if let Some(next_term) = aliases.get(&v) {
                // Sécurité anti-cycle (ex: v1 = v2 et v2 = v1)
                if !visited.insert(v) {
                    break;
                }
                current_term = next_term.clone();
            } else {
                break;
            }
        }

        // On "aplatit" la structure (Path Compression)
        if let Some(alias) = aliases.get_mut(&start_var) {
            *alias = current_term;
        }
    }
}

/// Résout une variable vers son représentant canonique (le plus petit ID du groupe d'égalité)
#[inline(always)]
pub(crate) fn resolve_var(
    v: VariableId,
    current_aliases: &std::collections::HashMap<VariableId, Term>, // 💡 Injecté à la place de self
) -> Term {
    // Si la fermeture a bien aplati la map, un seul get suffit.
    current_aliases
        .get(&v)
        .cloned()
        .unwrap_or(Term::Variable(v))
}
