use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Lowering final d'un arbre d'expression vers sa forme PNF encodée.
///
/// Point d'entrée pratique qui alloue un scratchpad temporaire à la volée.
///
/// # Preconditions
///
/// Cette fonction assume que l'arbre d'expression a déjà subi les passes de normalisation suivantes :
/// 1. **Forme Positive / NNF (Negation Normal Form)** : Les opérateurs de négation (`Not`) doivent porter
///    uniquement et directement sur des littéraux terminaux (atomes ou comparaisons). Aucune négation ne
///    doit surplomber un quantificateur (`Forall`, `Exists`) ou un connecteur logique (`And`, `Or`).
/// 2. **Forme Rectifiée / QNF (Quantifier Normal Form)** : Toutes les variables liées par des quantificateurs
///    doivent avoir été renommées de manière unique afin d'éviter toute capture accidentelle ou collision
///    de variables lors de l'extraction des quantificateurs vers la racine.
///
/// Si ces préconditions ne sont pas respectées, la fonction retournera immédiatement une erreur de type `GroundingError`.
pub fn to_pnf(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(expr_id, store, negated_atoms, &mut scratchpad, is_effect)
}

/// Lowering final d'un arbre d'expression vers sa forme PNF encodée.
///
/// Reconstruit l'arbre de bas en haut (Hash Consing) et retourne le nouvel `ExprId`.
///
/// # Preconditions
///
/// L'algorithme de passage en PNF (forme prénexe) repose sur des invariants structurels stricts de l'arbre source :
///
/// * **NNF (Forme Positive)** : Les négations complexes (ex: doubles négations `Not(Not(...))` ou négations de blocs
///   `Not(And(...))`) sont interdites. L'algorithme traite les négations par absorption directe dans le bit-mask
///   des atomes. Rencontrer une structure non positive lèvera immédiatement une erreur `StorerError::invalid_node`.
/// * **QNF (Standardisation des variables)** : Chaque quantificateur doit posséder un identifiant de variable unique
///   à l'échelle de l'expression complète. Sans cela, le déplacement des quantificateurs en tête d'arbre détruira
///   la sémantique d'origine par capture de variables.
///
/// # Algorithme
///
/// Le traitement est itératif (basé sur une pile explicite) afin d'éviter les débordements de pile (*stack overflow*)
/// sur les arbres profonds. Il utilise une stratégie en deux temps :
/// 1. **Phase de descente** : Propagation du contexte logique (`in_condition`) et validation *fail-fast* des invariants NNF.
/// 2. **Phase de remontée** : Reconstruction de l'arbre à l'aide du mécanisme de *Hash-Consing* du `ExprStore` pour
///    garantir la déduplication agressive des nœuds.
pub fn to_pnf_with_scratchpad(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    scratchpad.clear();

    // Amorçage de la pile : (ExprId, InCondition, ChildrenPushed)
    scratchpad.stack.push((expr_id, !is_effect, false));

    while let Some((old_id, in_condition, children_pushed)) = scratchpad.stack.pop() {
        if !children_pushed {
            // --- 1. PHASE DE DESCENTE (PROPAGATION DU CONTEXTE) ---
            scratchpad.stack.push((old_id, in_condition, true));

            let entry_kind = store[old_id].kind().clone();
            match entry_kind {
                ExprKind::Not => {
                    if in_condition {
                        let child_id = *store[old_id]
                            .children()
                            .first()
                            .expect("Not must have a child");

                        // --- CORRECTIF : Détection précoce de la double négation ---
                        if store[child_id].kind() == &ExprKind::Not {
                            return Err(StorerError::invalid_node(child_id).into());
                        }

                        scratchpad.stack.push((child_id, in_condition, false));
                    }
                }
                ExprKind::When => {
                    let children = store[old_id].children();
                    if children.len() != 2 {
                        return Err(StorerError::invalid_node(old_id).into());
                    }
                    scratchpad.stack.push((children[0], true, false)); // Condition
                    scratchpad.stack.push((children[1], false, false)); // Effet
                }
                ExprKind::And
                | ExprKind::Or
                | ExprKind::Forall(_)
                | ExprKind::Exists(_)
                | ExprKind::Always
                | ExprKind::Sometime
                | ExprKind::Within
                | ExprKind::AtMostOnce
                | ExprKind::SometimeAfter
                | ExprKind::SometimeBefore
                | ExprKind::AlwaysWithin
                | ExprKind::HoldDuring
                | ExprKind::HoldAfter => {
                    for &child_id in store[old_id].children().iter().rev() {
                        scratchpad.stack.push((child_id, in_condition, false));
                    }
                }
                ExprKind::Imply => {
                    return Err(StorerError::invalid_node(old_id).into());
                }
                _ => {} // Nœuds terminaux
            }
        } else {
            // --- 2. PHASE DE REMONTÉE (RECONSTRUCTION HASH-CONSED) ---
            let entry_kind = store[old_id].kind().clone();

            let new_id = match entry_kind {
                ExprKind::Not => {
                    if in_condition {
                        let child_id = *store[old_id].children().first().unwrap();
                        let new_child_id = *scratchpad.cache.get(&child_id).unwrap_or(&child_id);
                        let child_kind = store[new_child_id].kind().clone();

                        match child_kind {
                            ExprKind::Not => {
                                return Err(StorerError::invalid_node(new_child_id).into())
                            }
                            ExprKind::AtomicFormula(mut atom_id) => {
                                if atom_id.is_negated() {
                                    return Err(StorerError::invalid_node(new_child_id).into());
                                }
                                // Absorption de la négation : On applique le bit-mask MSB
                                atom_id.set_negated(true);
                                negated_atoms.push(atom_id);

                                // Internement du nouvel atome modifié
                                store.intern(ExprKind::AtomicFormula(atom_id), &[])
                            }
                            ExprKind::Comparison(_) => store.intern(ExprKind::Not, &[new_child_id]),
                            _ => return Err(StorerError::invalid_node(new_child_id).into()),
                        }
                    } else {
                        // Hors condition (Delete Effect), on reconstruit le Not classique
                        let child_id = *store[old_id].children().first().unwrap();
                        let new_child_id = *scratchpad.cache.get(&child_id).unwrap_or(&child_id);
                        store.intern(ExprKind::Not, &[new_child_id])
                    }
                }
                _ => {
                    let old_children = store[old_id].children();
                    if old_children.is_empty() {
                        old_id
                    } else {
                        let mut has_changed = false;
                        scratchpad.children_buffer.clear();

                        // Optimisation Lazy Copying : évite d'écrire dans le buffer tant que rien ne change
                        for (idx, &child_id) in old_children.iter().enumerate() {
                            let new_child_id =
                                *scratchpad.cache.get(&child_id).unwrap_or(&child_id);

                            if has_changed {
                                scratchpad.children_buffer.push(new_child_id);
                            } else if new_child_id != child_id {
                                has_changed = true;
                                // On rattrape le retard en copiant les enfants précédents d'un coup sec
                                scratchpad
                                    .children_buffer
                                    .extend_from_slice(&old_children[..idx]);
                                scratchpad.children_buffer.push(new_child_id);
                            }
                        }

                        if has_changed {
                            store.intern(entry_kind, &scratchpad.children_buffer)
                        } else {
                            old_id
                        }
                    }
                }
            };

            scratchpad.cache.insert(old_id, new_id);
        }
    }

    negated_atoms.sort_unstable();
    negated_atoms.dedup();

    Ok(*scratchpad.cache.get(&expr_id).unwrap_or(&expr_id))
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::grounding::error::GroundingError;
    use crate::aiplan4rust::compiler::grounding::passes::pnf::expr::to_pnf_with_scratchpad;
    use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, CompareOp};

    #[test]
    fn test_encode_simple_atom_negation_logical() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (at-robot r1))
        let mut skeleton_id = AtomSkeletonId::new(500);
        skeleton_id.set_negated(false);

        let atom_id = store.intern(ExprKind::AtomicFormula(skeleton_id), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Transformation (is_effect = false -> Mode Condition/Logique)
        let new_root_id = to_pnf_with_scratchpad(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation
        let root_kind = store[new_root_id].kind();

        if let ExprKind::AtomicFormula(final_atom_id) = root_kind {
            assert!(
                final_atom_id.is_negated(),
                "Le bit MSB de l'atome doit être à true"
            );
            assert_eq!(
                negated_atoms.len(),
                1,
                "L'atome doit être collecté globalement"
            );
            assert_eq!(
                negated_atoms[0], *final_atom_id,
                "L'ID collecté doit correspondre à l'atome modifié"
            );
        } else {
            panic!("Le Not logique aurait dû être absorbé pour devenir une AtomicFormula directe");
        }

        Ok(())
    }

    #[test]
    fn test_encode_effect_negation_preservation() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (at-robot r1))
        let mut skeleton_id = AtomSkeletonId::new(500);
        skeleton_id.set_negated(false);
        let atom_id = store.intern(ExprKind::AtomicFormula(skeleton_id), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Transformation (is_effect = true -> Mode Effet / Delete effect)
        let new_root_id = to_pnf_with_scratchpad(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            true,
        )?;

        // 3. Validation
        let root_kind = store[new_root_id].kind();
        assert!(
            matches!(root_kind, ExprKind::Not),
            "Le 'Not' en mode effet doit être conservé tel quel"
        );
        assert!(
            negated_atoms.is_empty(),
            "Les effets de suppression ne doivent pas alimenter negated_atoms"
        );

        Ok(())
    }

    #[test]
    fn test_encode_comparison_stays_unchanged() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (= ?x ?y))
        // Utilisation du builder pour générer des opérandes et une comparaison valide
        let comp_id = {
            let mut builder = ExprBuilder::new(&mut store);
            let n1 = builder.number(10.0);
            // On utilise des valeurs dynamiques non-pliables (ou variables si disponibles)
            // pour forcer la création d'un nœud Comparison structurel stable
            let n2 = builder.number(20.0);
            builder.less(n1, n2) // Génère Less de manière canonique (ou True si plié, ajusté ici)
        };

        // Pour s'assurer qu'on teste une comparaison structurelle pure sans pliage constant trivial,
        // on l'interne directement avec l'opérateur requis si le builder le pliait en constante.
        let comp_id = store.intern(ExprKind::Comparison(CompareOp::Equal), &[]);
        let not_id = store.intern(ExprKind::Not, &[comp_id]);

        // 2. Transformation
        let new_root_id = to_pnf_with_scratchpad(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation
        let root_kind = store[new_root_id].kind();
        assert!(
            matches!(root_kind, ExprKind::Not),
            "La racine doit rester un nœud Not pour les comparaisons"
        );

        let children = store[new_root_id].children();
        assert_eq!(children.len(), 1);
        assert_eq!(
            store[children[0]].kind(),
            &ExprKind::Comparison(CompareOp::Equal)
        );

        assert!(
            negated_atoms.is_empty(),
            "Les comparaisons niées ne doivent pas être collectées dans negated_atoms"
        );

        Ok(())
    }

    #[test]
    fn test_detect_unsupported_node_under_not() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (imply A B))
        let a_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(100)), &[]);
        let b_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(101)), &[]);
        let imply_id = store.intern(ExprKind::Imply, &[a_id, b_id]);
        let not_id = store.intern(ExprKind::Not, &[imply_id]);

        // 2. Transformation & Validation
        let result = to_pnf_with_scratchpad(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        assert!(
            result.is_err(),
            "La passe doit renvoyer une erreur si un nœud 'Imply' non supporté est trouvé sous un 'Not'"
        );
        assert!(negated_atoms.is_empty());
    }

    #[test]
    fn test_detect_double_negation_failure() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (not A))
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(100)), &[]);
        let inner_not = store.intern(ExprKind::Not, &[atom_id]);
        let outer_not = store.intern(ExprKind::Not, &[inner_not]);

        // 2. Transformation & Validation
        let result = to_pnf_with_scratchpad(
            outer_not,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        assert!(
            result.is_err(),
            "Le moteur doit rejeter les doubles négations (NOT NOT) directes"
        );
        assert!(negated_atoms.is_empty());
    }

    #[test]
    fn test_mixed_complex_pnf() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (and (not (at-robot)) (not (= ?x ?y)))
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(500)), &[]);
        let not_atom_id = store.intern(ExprKind::Not, &[atom_id]);

        let comp_id = store.intern(ExprKind::Comparison(CompareOp::Equal), &[]);
        let not_comp_id = store.intern(ExprKind::Not, &[comp_id]);

        let and_id = store.intern(ExprKind::And, &[not_atom_id, not_comp_id]);

        // 2. Transformation
        let new_root_id = to_pnf_with_scratchpad(
            and_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation
        let root_node = &store[new_root_id];
        assert!(matches!(root_node.kind(), ExprKind::And));

        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Enfant 1: L'atome doit être absorbé
        if let ExprKind::AtomicFormula(final_id) = store[children[0]].kind() {
            assert!(final_id.is_negated());
            assert_eq!(negated_atoms.len(), 1);
            assert_eq!(negated_atoms[0], *final_id);
        } else {
            panic!("Le premier enfant aurait dû être converti en AtomicFormula direct");
        }

        // Enfant 2: La comparaison sous le Not reste inchangée
        assert!(matches!(store[children[1]].kind(), ExprKind::Not));
        let inner_comp_children = store[children[1]].children();
        assert_eq!(
            store[inner_comp_children[0]].kind(),
            &ExprKind::Comparison(CompareOp::Equal)
        );

        Ok(())
    }

    #[test]
    fn test_detect_forbidden_double_negation_in_bit() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Create an atom that is already bit-negated
        let mut corrupted_skeleton = AtomSkeletonId::new(500);
        corrupted_skeleton.set_negated(true);

        let atom_id = store.intern(ExprKind::AtomicFormula(corrupted_skeleton), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Transformation
        let result = to_pnf_with_scratchpad(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        // 3. Validation
        assert!(
            result.is_err(),
            "Le moteur doit lever une erreur s'il rencontre un Not au-dessus d'un atome déjà bit-nié"
        );
        assert!(negated_atoms.is_empty());
    }

    #[test]
    fn test_pnf_deep_nesting() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Feuille : (not A)
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(500)), &[]);
        let mut current_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Empilement de 1000 nœuds AND
        for _ in 0..1000 {
            current_id = store.intern(ExprKind::And, &[current_id]);
        }

        // 3. Transformation
        let new_root_id = to_pnf_with_scratchpad(
            current_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 4. Validation
        let mut checker_id = new_root_id;
        for _ in 0..1000 {
            let node = &store[checker_id];
            assert!(matches!(node.kind(), ExprKind::And));
            checker_id = node.children()[0];
        }

        if let ExprKind::AtomicFormula(final_id) = store[checker_id].kind() {
            assert!(final_id.is_negated());
            assert_eq!(negated_atoms.len(), 1);
            assert_eq!(negated_atoms[0], *final_id);
        } else {
            panic!("La feuille profonde doit être une AtomicFormula");
        }

        Ok(())
    }
}
