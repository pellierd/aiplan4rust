use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::ops::rewriting::TimeSpecifier;
use crate::aiplan4rust::lir::store::{ExprBuilder, ExprEntryKind, ExprId};

pub fn push_time_specifier(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    scratch.clear();

    // 1. On packe la racine (ID + None)
    let root_packed = TimeSpecifier::None.pack(expr.as_usize());
    // On utilise un ID "fictif" pour le scratch car il stocke des ID,
    // ou on adapte le scratch pour prendre des usize.
    scratch.push(ExprId::from(root_packed), false);

    while let Some((packed_id, processed)) = scratch.pop() {
        // 2. On utilise la méthode unpack de lang
        let (curr_id_raw, context) = TimeSpecifier::unpack(packed_id.as_usize());
        let curr_id = ExprId::from(curr_id_raw);

        if scratch.get(packed_id).is_some() && !processed {
            continue;
        }

        let entry = builder.fetch(curr_id)?;
        let children = entry.children();

        if processed {
            // --- RECONSTRUCTION (Bottom-Up) ---
            // --- RECONSTRUCTION (Bottom-Up) ---
            let new_id = match entry.kind() {
                // 1. Si on croise un specifier, on a déjà traité son enfant avec le bon contexte
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                    let next_ctx = match entry.kind() {
                        ExprEntryKind::AtStart => TimeSpecifier::AtStart,
                        ExprEntryKind::AtEnd => TimeSpecifier::AtEnd,
                        _ => TimeSpecifier::Overall,
                    };
                    scratch.fetch(ExprId::from(next_ctx.pack(children[0].as_usize())))
                }

                // 2. Connecteurs logiques : on reconstruit normalement
                ExprEntryKind::And
                | ExprEntryKind::Or
                | ExprEntryKind::Not
                | ExprEntryKind::Imply
                | ExprEntryKind::When
                | ExprEntryKind::Forall(_)
                | ExprEntryKind::Exists(_) => {
                    let new_children: Vec<ExprId> = children
                        .iter()
                        .map(|&c| scratch.fetch(ExprId::from(context.pack(c.as_usize()))))
                        .collect();

                    // Utilise reconstruct ici pour la propreté si tu veux
                    builder.reconstruct(entry.kind().clone(), new_children)
                }

                // 3. TERMINAUX (Atomes, etc.) : C'est ici que ça cassait !
                kind => {
                    // On ne crée pas l'atome seul si on a un contexte.
                    // On utilise les méthodes du builder qui créent l'atome ET le specifier d'un coup
                    // ou on s'assure que le builder ne valide pas l'atome nu.
                    match context {
                        TimeSpecifier::AtStart => {
                            let atom = builder.intern(kind.clone(), children.to_vec());
                            builder.at_start(atom)
                        }
                        TimeSpecifier::AtEnd => {
                            let atom = builder.intern(kind.clone(), children.to_vec());
                            builder.at_end(atom)
                        }
                        TimeSpecifier::Overall => {
                            let atom = builder.intern(kind.clone(), children.to_vec());
                            builder.overall(atom)
                        }
                        TimeSpecifier::None => {
                            // Si on arrive ici sans contexte pour un atome,
                            // c'est là que l'erreur MissingTimeSpecifier est légitime.
                            builder.intern(kind.clone(), children.to_vec())
                        }
                    }
                }
            };

            scratch.insert(packed_id, new_id);
        } else {
            // --- DESCENTE (Top-Down) ---
            scratch.push(packed_id, true);

            match entry.kind() {
                ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                    let next_ctx = match entry.kind() {
                        ExprEntryKind::AtStart => TimeSpecifier::AtStart,
                        ExprEntryKind::AtEnd => TimeSpecifier::AtEnd,
                        _ => TimeSpecifier::Overall,
                    };
                    scratch.push(ExprId::from(next_ctx.pack(children[0].as_usize())), false);
                }
                _ => {
                    for &child in children.iter().rev() {
                        scratch.push(ExprId::from(context.pack(child.as_usize())), false);
                    }
                }
            }
        }
    }

    let final_id = scratch.fetch(ExprId::from(root_packed));

    #[cfg(debug_assertions)]
    {
        check_temporal_consistency(final_id, builder, scratch)?;
    }

    Ok(final_id)
}

pub fn check_temporal_consistency(
    root: ExprId,
    builder: &ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<(), ExprOpErrorHC> {
    scratch.clear();

    // 1. Initialisation avec le pack racine
    let root_packed = TimeSpecifier::None.pack(root.as_usize());
    scratch.push(ExprId::from(root_packed), false);

    while let Some((packed_id, _)) = scratch.pop() {
        // 2. Déballage via l'API de TimeSpecifier
        let (curr_id_raw, context) = TimeSpecifier::unpack(packed_id.as_usize());
        let curr_id = ExprId::from(curr_id_raw);

        if scratch.get(packed_id).is_some() {
            continue;
        }

        let entry = builder.fetch(curr_id)?;

        match entry.kind() {
            // --- 1. NOEUDS TEMPORELS ---
            // On change de contexte pour les enfants
            ExprEntryKind::AtStart | ExprEntryKind::AtEnd | ExprEntryKind::Overall => {
                let next_ctx = match entry.kind() {
                    ExprEntryKind::AtStart => TimeSpecifier::AtStart,
                    ExprEntryKind::AtEnd => TimeSpecifier::AtEnd,
                    _ => TimeSpecifier::Overall,
                };
                if let Some(&child) = entry.children().first() {
                    scratch.push(ExprId::from(next_ctx.pack(child.as_usize())), false);
                }
            }

            // --- 2. OPÉRATEURS LOGIQUES & STRUCTURELS ---
            // Ils propagent le contexte actuel (context) à leurs enfants.
            ExprEntryKind::And
            | ExprEntryKind::Or
            | ExprEntryKind::Not
            | ExprEntryKind::Imply
            | ExprEntryKind::When
            | ExprEntryKind::Forall(_)
            | ExprEntryKind::Exists(_)
            | ExprEntryKind::Metric(_)
            | ExprEntryKind::Preference
            | ExprEntryKind::LabeledTask
            | ExprEntryKind::Serial
            | ExprEntryKind::Parallel => {
                for &child in entry.children() {
                    scratch.push(ExprId::from(context.pack(child.as_usize())), false);
                }
            }

            // --- 3. COMPOSANTS INTERNES (SANS TEMPS REQUIS) ---
            // Ces éléments sont des feuilles ou des définitions structurelles.
            ExprEntryKind::PredicateSymbol(_)
            | ExprEntryKind::TaskSymbol(_)
            | ExprEntryKind::FunctionSymbol(_)
            | ExprEntryKind::PrefName(_)
            | ExprEntryKind::Variable(_)
            | ExprEntryKind::Object(_)
            | ExprEntryKind::Number(_)
            | ExprEntryKind::TotalTime
            | ExprEntryKind::Length
            | ExprEntryKind::TaskLabel(_)
            | ExprEntryKind::TaskOrderingConstraint(_) => {
                // Rien à faire : ces nœuds sont valides même sans contexte temporel.
            }

            // --- 4. LITTÉRAUX ET FAITS (DOIVENT AVOIR UN CONTEXTE) ---
            _ => {
                if context == TimeSpecifier::None {
                    // On utilise ton erreur typée avec le helper de traçage
                    return Err(ExprOpErrorHC::missing_time_specifier(
                        curr_id,
                        entry.kind().clone(),
                    ));
                }

                // Pour les arguments des faits (ex: les objets dans (p ?a ?b)),
                // on repasse en contexte None car le temps qualifie le prédicat, pas ses termes.
                for &child in entry.children() {
                    scratch.push(
                        ExprId::from(TimeSpecifier::None.pack(child.as_usize())),
                        false,
                    );
                }
            }
        }

        // Marquer comme visité dans le scratchpad
        scratch.insert(packed_id, curr_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::store::iter::Scratchpad;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Test la distribution du spécificateur temporel à travers un nœud logique AND.
    /// On vérifie que `(at start (and A B))` devient `(and (at start A) (at start B))`.
    #[test]
    fn test_push_at_start_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        // INPUT    : (at start (and A B))
        // EXPECTED : (and (at start A) (at start B))
        let a = builder.atomic_formula(1, vec![], 100);
        let b = builder.atomic_formula(2, vec![], 101);
        let and_node = builder.and(vec![a, b]);
        let root = builder.at_start(and_node);

        let result_id = push_time_specifier(root, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;
        assert!(matches!(root_node.kind(), ExprEntryKind::And));

        for &child_id in root_node.children() {
            let child_node = builder.fetch(child_id)?;
            assert!(matches!(child_node.kind(), ExprEntryKind::AtStart));
            let inner_id = child_node.children()[0];
            assert!(inner_id == a || inner_id == b);
        }
        Ok(())
    }

    /// Test la propagation du temps à l'intérieur d'un quantificateur universel.
    /// On vérifie que le temps est poussé sur le corps de la formule sans altérer les variables.
    /// INPUT    : (at end (forall (?x) A))
    /// EXPECTED : (forall (?x) (at end A))
    #[test]
    fn test_push_at_end_forall() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = builder.atomic_formula(1, vec![], 100);
        let var_x = builder.typed_variable(10, &[100]);
        let vars = builder.typed_variable_list(vec![var_x]);
        let forall_node = builder.forall(vars, a);
        let root = builder.at_end(forall_node);

        let result_id = push_time_specifier(root, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;
        assert!(matches!(root_node.kind(), ExprEntryKind::Forall(_)));

        let body_id = root_node.children()[0];
        let body_node = builder.fetch(body_id)?;
        assert!(matches!(body_node.kind(), ExprEntryKind::AtEnd));
        assert_eq!(body_node.children()[0], a);

        Ok(())
    }

    /// Test la condition d'arrêt sur une formule atomique (feuille).
    /// Le temps ne peut pas être poussé plus loin qu'un prédicat de base.
    /// INPUT    : (overall A)
    /// EXPECTED : (overall A)  (Statu quo)
    #[test]
    fn test_push_overall_atomic() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let a = builder.atomic_formula(1, vec![], 100);
        let root = builder.overall(a);

        let result_id = push_time_specifier(root, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;
        assert!(matches!(root_node.kind(), ExprEntryKind::Overall));
        assert_eq!(root_node.children()[0], a);

        Ok(())
    }

    /// Test la distribution sur un effet conditionnel (nœud binaire complexe).
    /// On vérifie que le temps est appliqué à la fois à la condition et à l'effet.
    /// INPUT    : (at end (when Cond Eff))
    /// EXPECTED : (when (at end Cond) (at end Eff))
    #[test]
    fn test_push_at_end_when() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let cond = builder.atomic_formula(1, vec![], 100);
        let eff = builder.atomic_formula(2, vec![], 101);
        let when_node = builder.when(cond, eff);
        let root = builder.at_end(when_node);

        let result_id = push_time_specifier(root, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;

        assert!(matches!(root_node.kind(), ExprEntryKind::When));

        // Vérification condition
        let res_cond = builder.fetch(root_node.children()[0])?;
        assert!(matches!(res_cond.kind(), ExprEntryKind::AtEnd));

        // Vérification effet
        let res_eff = builder.fetch(root_node.children()[1])?;
        assert!(matches!(res_eff.kind(), ExprEntryKind::AtEnd));

        Ok(())
    }
}
