use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::ops::rewriting::push_time_specifier;
use crate::aiplan4rust::lir::store::{ExprBuilder, ExprEntryKind, ExprId};
use ahash::{HashMap, HashMapExt};

pub fn factorize_time_specifier(
    root: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    // 1. Prétraitement indispensable
    let pushed_root = push_time_specifier(root, builder, scratch)?;

    // 2. Initialisation du stockage des résultats
    // On va stocker pour chaque ExprId -> (StartId, EndId, OverallId)
    let mut triplet_map: HashMap<ExprId, (ExprId, ExprId, ExprId)> = HashMap::new();
    let empty = builder.empty_and();

    scratch.clear();
    scratch.push(pushed_root, false);

    while let Some((curr_id, processed)) = scratch.pop() {
        if processed {
            let (kind, children) = {
                let entry = builder.fetch(curr_id)?;
                (entry.kind().clone(), entry.children().to_vec())
            };

            let res_triplet = match kind {
                // --- CAS DES MARQUEURS ---
                ExprEntryKind::AtStart => {
                    let child_triplet = triplet_map[&children[0]];
                    // On ne garde que la dimension "Start" du contenu
                    (child_triplet.0, empty, empty)
                }
                ExprEntryKind::AtEnd => {
                    let child_triplet = triplet_map[&children[0]];
                    // On ne garde que la dimension "End" du contenu
                    (empty, child_triplet.1, empty)
                }
                ExprEntryKind::Overall => {
                    let child_triplet = triplet_map[&children[0]];
                    // On ne garde que la dimension "Overall" du contenu
                    (empty, empty, child_triplet.2)
                }

                // --- CAS DES CONNECTEURS LOGIQUES ---
                k @ (ExprEntryKind::And
                | ExprEntryKind::Or
                | ExprEntryKind::Not
                | ExprEntryKind::Forall(_)
                | ExprEntryKind::Exists(_)) => {
                    let mut starts = Vec::new();
                    let mut ends = Vec::new();
                    let mut overalls = Vec::new();

                    for &c in &children {
                        let (s, e, o) = triplet_map[&c];
                        if s != empty {
                            starts.push(s);
                        }
                        if e != empty {
                            ends.push(e);
                        }
                        if o != empty {
                            overalls.push(o);
                        }
                    }

                    // On reconstruit le triplet pour ce noeud
                    (
                        rebuild_safe(builder, &k, starts, empty),
                        rebuild_safe(builder, &k, ends, empty),
                        rebuild_safe(builder, &k, overalls, empty),
                    )
                }

                // --- CAS DES FEUILLES (ATOME) ---
                other => {
                    let id = builder.reconstruct(other, children);
                    (id, id, id) // Un atome "nu" appartient virtuellement aux 3
                }
            };

            triplet_map.insert(curr_id, res_triplet);
        } else {
            let entry = builder.fetch(curr_id)?;
            let children = entry.children().to_vec();
            scratch.push(curr_id, true);
            for &child in children.iter().rev() {
                scratch.push(child, false);
            }
        }
    }

    // 3. Assemblage final
    let (final_s, final_e, final_o) = triplet_map[&pushed_root];

    let branch_s = builder.at_start(final_s);
    let branch_e = builder.at_end(final_e);
    let branch_o = builder.overall(final_o);

    Ok(builder.and(vec![branch_s, branch_e, branch_o]))
}

/// Helper pour reconstruire sans imbrications inutiles
fn rebuild_safe(
    builder: &mut ExprBuilder,
    kind: &ExprEntryKind,
    children: Vec<ExprId>,
    empty: ExprId,
) -> ExprId {
    if children.is_empty() {
        empty
    } else if children.len() == 1 && matches!(kind, ExprEntryKind::And | ExprEntryKind::Or) {
        children[0]
    } else {
        builder.reconstruct(kind.clone(), children)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::store::iter::Scratchpad;
    use crate::aiplan4rust::lir::store::{ExprBuilder, ExprStore};

    #[test]
    fn test_normalize_temporal_complex_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Création des atomes
        let a = builder.atomic_formula(1, vec![], 0);
        let b = builder.atomic_formula(2, vec![], 0);
        let c = builder.atomic_formula(3, vec![], 0);

        // 2. Construction de l'entrée : (or (and (at-start A) (overall B)) (at-end (not C)))
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let and_start_overall = builder.and(vec![start_a, overall_b]);

        let not_c = builder.not(c);
        let end_not_c = builder.at_end(not_c);

        let root = builder.or(vec![and_start_overall, end_not_c]);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Structure plate)
        let empty = builder.empty_and();

        // On reconstruit les branches pour l'ID attendu
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(not_c); // not_c déjà créé plus haut
        let expected_overall = builder.overall(b);

        let expected_vec = vec![expected_start, expected_end, expected_overall];
        let expected = builder.and(expected_vec);

        // 5. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_normalize_temporal_negation() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Création de l'atome
        let a = builder.atomic_formula(1, vec![], 0);

        // 2. Construction de l'entrée : (not (at-start A))
        // Note : factorize_time_specifier suppose que push_time_specifier a déjà été appelé
        // ou gère la négation en interne.
        let start_a = builder.at_start(a);
        let root = builder.not(start_a);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Step-by-step)
        let empty = builder.empty_and();
        let not_a = builder.not(a);

        let expected_start = builder.at_start(not_a);
        let expected_end = builder.at_end(empty);
        let expected_overall = builder.overall(empty);

        let expected_vec = vec![expected_start, expected_end, expected_overall];
        let expected = builder.and(expected_vec);

        // 5. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_normalize_temporal_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Création des atomes
        let a = builder.atomic_formula(1, vec![], 0);
        let b = builder.atomic_formula(2, vec![], 0);
        let c = builder.atomic_formula(3, vec![], 0);

        // 2. Construction de l'entrée : (and (at-start A) (or (overall B) (at-end C)))
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let end_c = builder.at_end(c);

        let or_inner = builder.or(vec![overall_b, end_c]);
        let root = builder.and(vec![start_a, or_inner]);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Structure plate distribuée)
        // On s'attend à ce que le OR disparaisse au profit de la distribution temporelle
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(c);
        let expected_overall = builder.overall(b);

        let expected_vec = vec![expected_start, expected_end, expected_overall];
        let expected = builder.and(expected_vec);

        // 5. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_normalize_empty_contexts() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Création de l'atome
        let a = builder.atomic_formula(1, vec![], 0);

        // 2. Construction de la racine d'entrée
        let root = builder.at_start(a);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Step-by-step)
        let empty = builder.empty_and();

        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(empty);
        let expected_overall = builder.overall(empty);

        let expected_vec = vec![expected_start, expected_end, expected_overall];

        let expected = builder.and(expected_vec);

        // 5. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_normalize_temporal_atstart_forall() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Atomes et variables
        let a = builder.atomic_formula(1, vec![], 0);
        let b = builder.atomic_formula(2, vec![], 0);
        let var_x = builder.typed_variable(10, &[100]);

        // 2. Construction logique (Forall)
        let variables = vec![var_x].into();
        let forall_a = builder.forall(variables, a);

        // 3. Application des marqueurs temporels
        let start_forall = builder.at_start(forall_a);
        let end_b = builder.at_end(b);

        // 4. Construction de la racine d'entrée
        let root_vec = vec![start_forall, end_b];
        let root = builder.and(root_vec);

        // 5. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 6. Construction de l'attendu (Step-by-step)
        let empty_and = builder.empty_and();
        let overall_empty = builder.overall(empty_and);

        // On réutilise les IDs déjà créés pour garantir le Hash Consing
        let expected_vec = vec![start_forall, end_b, overall_empty];
        let expected = builder.and(expected_vec);

        // 7. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_normalize_mixed_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::new();
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Création des atomes
        let a = builder.atomic_formula(1, vec![], 0);
        let b = builder.atomic_formula(2, vec![], 0);

        // 2. Création des sous-nœuds temporels (étape par étape)
        let start_a = builder.at_start(a);
        let end_b = builder.at_end(b);

        // 3. Construction de la racine
        let root = builder.and(vec![start_a, end_b]);

        // 4. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 5. Construction de l'attendu (étape par étape aussi)
        let empty = builder.empty_and();
        let overall_empty = builder.overall(empty);

        // On recrée les morceaux pour l'attendu
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(b);

        let expected = builder.and(vec![expected_start, expected_end, overall_empty]);

        // 6. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }
}
