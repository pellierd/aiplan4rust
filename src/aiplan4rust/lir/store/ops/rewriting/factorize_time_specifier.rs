/*pub fn factorize_time_specifier(
    root: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    // 1. Prétraitement indispensable (pousse les marqueurs vers les feuilles)
    let pushed_root = push_time_specifier(root, builder, scratch)?;

    // 2. Initialisation
    // On utilise désormais les capacités du scratchpad pour éviter les allocations locales
    scratch.clear();
    let empty = builder.empty_and();

    scratch.push(pushed_root, false);

    while let Some((curr_id, processed)) = scratch.pop() {
        let entry = builder.fetch(curr_id)?;
        let children = entry.children();

        if processed {
            let kind = entry.kind();

            let res_triplet = match kind {
                // --- CAS DES MARQUEURS ---
                ExprEntryKind::AtStart => {
                    let (s, _, _) = scratch.get_temporal_decomposition(children[0].as_usize());
                    (s, empty, empty)
                }
                ExprEntryKind::AtEnd => {
                    let (_, e, _) = scratch.get_temporal_decomposition(children[0].as_usize());
                    (empty, e, empty)
                }
                ExprEntryKind::Overall => {
                    let (_, _, o) = scratch.get_temporal_decomposition(children[0].as_usize());
                    (empty, empty, o)
                }

                // --- CAS DES CONNECTEURS LOGIQUES ---
                ExprEntryKind::And
                | ExprEntryKind::Or
                | ExprEntryKind::Not
                | ExprEntryKind::Forall(_)
                | ExprEntryKind::Exists(_) => {
                    // On utilise les buffers dédiés du scratchpad au lieu de Vec locaux
                    scratch.clear_time_specifier_buffers();

                    for &c in children {
                        let (s, e, o) = scratch.get_temporal_decomposition(c.as_usize());
                        scratch.push_time_specifier(s, e, o, empty);
                    }

                    (
                        rebuild_safe(builder, kind, scratch.collected_starts(), empty),
                        rebuild_safe(builder, kind, scratch.collected_ends(), empty),
                        rebuild_safe(builder, kind, scratch.collected_overalls(), empty),
                    )
                }

                // --- CAS DES FEUILLES (ATOME, etc.) ---
                other => {
                    // builder.intern avec la slice directe du store
                    let id = builder.intern(other.clone(), children);
                    (id, id, id)
                }
            };

            // On stocke dans le temporal_cache (FxHashMap<usize, ...>) du scratchpad
            scratch.save_temporal_decomposition(curr_id.as_usize(), res_triplet);
        } else {
            // --- PHASE DESCENTE ---
            scratch.push(curr_id, true);
            for &child in children.iter().rev() {
                scratch.push(child, false);
            }
        }
    }

    // 3. Assemblage final
    let (final_s, final_e, final_o) = scratch.get_temporal_decomposition(pushed_root.as_usize());

    let branch_s = builder.at_start(final_s);
    let branch_e = builder.at_end(final_e);
    let branch_o = builder.overall(final_o);

    // builder.and sur une slice de pile (stack)
    Ok(builder.and(&[branch_s, branch_e, branch_o]))
}

/// Helper mis à jour pour accepter les slices
fn rebuild_safe(
    builder: &mut ExprBuilder,
    kind: &ExprEntryKind,
    children: &[ExprId],
    empty: ExprId,
) -> ExprId {
    if children.is_empty() {
        empty
    } else if children.len() == 1 && matches!(kind, ExprEntryKind::And | ExprEntryKind::Or) {
        children[0]
    } else {
        // builder.reconstruct attend une slice
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
        let a = builder.atomic_formula(1, &[], 0);
        let b = builder.atomic_formula(2, &[], 0);
        let c = builder.atomic_formula(3, &[], 0);

        // 2. Construction de l'entrée : (or (and (at-start A) (overall B)) (at-end (not C)))
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let and_start_overall = builder.and(&[start_a, overall_b]);

        let not_c = builder.not(c);
        let end_not_c = builder.at_end(not_c);

        let root = builder.or(&[and_start_overall, end_not_c]);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Structure plate)
        let empty = builder.empty_and();

        // On reconstruit les branches pour l'ID attendu
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(not_c); // not_c déjà créé plus haut
        let expected_overall = builder.overall(b);

        let expected_vec = &[expected_start, expected_end, expected_overall];
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
        let a = builder.atomic_formula(1, &[], 0);

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

        let expected_vec = &[expected_start, expected_end, expected_overall];
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
        let a = builder.atomic_formula(1, &[], 0);
        let b = builder.atomic_formula(2, &[], 0);
        let c = builder.atomic_formula(3, &[], 0);

        // 2. Construction de l'entrée : (and (at-start A) (or (overall B) (at-end C)))
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let end_c = builder.at_end(c);

        let or_inner = builder.or(&[overall_b, end_c]);
        let root = builder.and(&[start_a, or_inner]);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Structure plate distribuée)
        // On s'attend à ce que le OR disparaisse au profit de la distribution temporelle
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(c);
        let expected_overall = builder.overall(b);

        let expected_vec = &[expected_start, expected_end, expected_overall];
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
        let a = builder.atomic_formula(1, &[], 0);

        // 2. Construction de la racine d'entrée
        let root = builder.at_start(a);

        // 3. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 4. Construction de l'attendu (Step-by-step)
        let empty = builder.empty_and();

        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(empty);
        let expected_overall = builder.overall(empty);

        let expected_vec = &[expected_start, expected_end, expected_overall];

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
        let a = builder.atomic_formula(1, &[], 0);
        let b = builder.atomic_formula(2, &[], 0);
        let var_x = builder.typed_variable(10, &[100]);

        // 2. Construction logique (Forall)
        let variables = vec![var_x].into();
        let forall_a = builder.forall(variables, a);

        // 3. Application des marqueurs temporels
        let start_forall = builder.at_start(forall_a);
        let end_b = builder.at_end(b);

        // 4. Construction de la racine d'entrée
        let root_vec = &[start_forall, end_b];
        let root = builder.and(root_vec);

        // 5. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 6. Construction de l'attendu (Step-by-step)
        let empty_and = builder.empty_and();
        let overall_empty = builder.overall(empty_and);

        // On réutilise les IDs déjà créés pour garantir le Hash Consing
        let expected_vec = &[start_forall, end_b, overall_empty];
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
        let a = builder.atomic_formula(1, &[], 0);
        let b = builder.atomic_formula(2, &[], 0);

        // 2. Création des sous-nœuds temporels (étape par étape)
        let start_a = builder.at_start(a);
        let end_b = builder.at_end(b);

        // 3. Construction de la racine
        let root = builder.and(&[start_a, end_b]);

        // 4. Transformation
        let result = factorize_time_specifier(root, &mut builder, &mut scratch)?;

        // 5. Construction de l'attendu (étape par étape aussi)
        let empty = builder.empty_and();
        let overall_empty = builder.overall(empty);

        // On recrée les morceaux pour l'attendu
        let expected_start = builder.at_start(a);
        let expected_end = builder.at_end(b);

        let expected = builder.and(&[expected_start, expected_end, overall_empty]);

        // 6. Comparaison finale
        assert_eq!(result, expected);
        Ok(())
    }
}*/
