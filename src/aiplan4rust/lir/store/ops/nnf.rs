use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

use crate::aiplan4rust::lir::store::builder::ExprBuilder;
/// Transforme une expression en Forme Normale de Négation (NNF).
/// Utilise le bit-packing pour réutiliser le Scratchpad sans allocations locales.
use smallvec::SmallVec;

pub fn to_nnf(
    root_id: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    scratch.clear();

    // --- BUFFERS LOCAUX RÉUTILISABLES ---
    // build_buf : pour construire les nouveaux nœuds (ex: après De Morgan)
    let mut build_buf: SmallVec<[ExprId; 32]> = SmallVec::new();
    // children_ids : pour extraire les enfants du store sans allocation
    let mut children_ids: SmallVec<[ExprId; 16]> = SmallVec::new();

    let root_encoded = ExprId::from(encode(root_id, false));
    scratch.push(root_encoded, false);

    while let Some((encoded_id, processed)) = scratch.pop() {
        let (curr_id, negate) = decode(encoded_id.as_usize());

        if scratch.get(encoded_id).is_some() && !processed {
            continue;
        }

        // --- PHASE 1 : EXTRACTION (Libère le builder) ---
        let kind = {
            let entry = builder.fetch(curr_id)?;

            children_ids.clear();
            children_ids.extend_from_slice(entry.children());

            entry.kind().clone()
        };

        if processed {
            // --- PHASE 2 : RECONSTRUCTION (Bottom-Up) ---
            let new_id = match &kind {
                ExprEntryKind::Not => scratch.fetch(ExprId::from(encode(children_ids[0], !negate))),

                ExprEntryKind::And | ExprEntryKind::Or => {
                    let is_and = matches!(kind, ExprEntryKind::And);

                    build_buf.clear();
                    for &c in &children_ids {
                        build_buf.push(scratch.fetch(ExprId::from(encode(c, negate))));
                    }

                    if is_and {
                        if negate {
                            builder.or(&build_buf)
                        } else {
                            builder.and(&build_buf)
                        }
                    } else {
                        if negate {
                            builder.and(&build_buf)
                        } else {
                            builder.or(&build_buf)
                        }
                    }
                }

                ExprEntryKind::Forall(vars) => {
                    let body = scratch.fetch(ExprId::from(encode(children_ids[0], negate)));
                    if negate {
                        builder.exists(vars.clone(), body)?
                    } else {
                        builder.forall(vars.clone(), body)?
                    }
                }
                ExprEntryKind::Exists(vars) => {
                    let body = scratch.fetch(ExprId::from(encode(children_ids[0], negate)));
                    if negate {
                        builder.forall(vars.clone(), body)?
                    } else {
                        builder.exists(vars.clone(), body)?
                    }
                }

                _ => {
                    let base = builder.intern(kind, &children_ids);
                    if negate {
                        builder.not(base)
                    } else {
                        base
                    }
                }
            };

            scratch.insert(encoded_id, new_id);
        } else {
            // --- PHASE 3 : DESCENTE (Top-Down) ---
            scratch.push(encoded_id, true);

            if matches!(kind, ExprEntryKind::Not) {
                scratch.push(ExprId::from(encode(children_ids[0], !negate)), false);
            } else {
                for &child in children_ids.iter().rev() {
                    scratch.push(ExprId::from(encode(child, negate)), false);
                }
            }
        }
    }

    Ok(scratch.fetch(root_encoded))
}

/// Encode un ExprId et sa polarité dans un seul usize.
/// Pair = Positif, Impair = Négatif.
#[inline(always)]
fn encode(id: ExprId, negate: bool) -> usize {
    let val = id.as_usize() << 1;
    if negate {
        val | 1
    } else {
        val
    }
}

/// Décode la valeur brute provenant du Scratchpad.
#[inline(always)]
fn decode(val: usize) -> (ExprId, bool) {
    (ExprId::from(val >> 1), (val & 1) == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::AtomSkeletonId;
    use crate::aiplan4rust::lir::store::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::iter::Scratchpad;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Test pushing negation through AND using De Morgan's law.
    /// Input: (not (and (A) (B))) -> (or (not (A)) (not (B)))
    #[test]
    fn test_push_negation_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∧ B)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let and_node = builder.and(&[a, b]);
        let root = builder.not(and_node);

        // 2. Transformation: De Morgan's Law ¬(A ∧ B) -> (¬A ∨ ¬B)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // On attend un OR à la racine
        assert!(matches!(root_node.kind(), ExprEntryKind::Or));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Vérification robuste : chaque enfant doit être un NOT
        // et les feuilles doivent être nos atomes d'origine
        let mut found_not_a = false;
        let mut found_not_b = false;

        for &child_id in children {
            let child_node = builder.fetch(child_id)?;
            assert!(matches!(child_node.kind(), ExprEntryKind::Not));

            let leaf_id = child_node.children()[0];
            if leaf_id == a {
                found_not_a = true;
            } else if leaf_id == b {
                found_not_b = true;
            }
        }

        assert!(found_not_a, "Not(A) est manquant dans le résultat");
        assert!(found_not_b, "Not(B) est manquant dans le résultat");

        Ok(())
    }

    /// Test pushing negation through OR using De Morgan's law.
    /// Input: (not (or (A) (B))) -> (and (not (A)) (not (B)))
    #[test]
    fn test_push_negation_or() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∨ B)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let or_node = builder.or(&[a, b]);
        let root = builder.not(or_node);

        // 2. Transformation: De Morgan's Law ¬(A ∨ B) -> (¬A ∧ ¬B)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // La racine doit être un AND
        assert!(matches!(root_node.kind(), ExprEntryKind::And));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Vérification robuste (le Store peut trier les IDs)
        let mut found_not_a = false;
        let mut found_not_b = false;

        for &child_id in children {
            let child_node = builder.fetch(child_id)?;
            assert!(matches!(child_node.kind(), ExprEntryKind::Not));

            let leaf_id = child_node.children()[0];
            if leaf_id == a {
                found_not_a = true;
            } else if leaf_id == b {
                found_not_b = true;
            }
        }

        assert!(found_not_a, "Not(A) est manquant dans la conjonction");
        assert!(found_not_b, "Not(B) est manquant dans la conjonction");

        Ok(())
    }

    /// Test pushing negation through a Forall quantifier.
    /// Input: (not (forall x (A))) -> (exists x (not (A)))
    #[test]
    fn test_push_negation_forall() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(forall (?X) (A))
        let a = builder.atomic_formula(1, &[], skel);
        let var_x = builder.typed_variable(10, &[100]); // ID 10, Type 100
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_node = builder.forall(forall_vars, a)?;
        let root = builder.not(forall_node);

        // 2. Transformation: ¬∀x.A -> ∃x.¬A
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // Le Forall sous négation doit être devenu un Exists
        assert!(matches!(root_node.kind(), ExprEntryKind::Exists(_)));

        // On vérifie que les variables sont conservées (même liste)
        if let ExprEntryKind::Exists(ref vars) = root_node.kind() {
            assert_eq!(vars.len(), 1);
            // On pourrait vérifier l'ID de la variable ici si nécessaire
        }

        // Le corps de l'Exists doit être ¬A
        let body_id = root_node.children()[0];
        let body_node = builder.fetch(body_id)?;
        assert!(matches!(body_node.kind(), ExprEntryKind::Not));

        let inner_atom_id = body_node.children()[0];
        assert_eq!(
            inner_atom_id, a,
            "L'atome à l'intérieur de la négation a été altéré"
        );

        Ok(())
    }

    /// Test pushing negation through an Exists quantifier.
    /// Input: (not (exists x (A))) -> (forall x (not (A)))
    #[test]
    fn test_push_negation_exists() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(exists (?X) (A))
        let a = builder.atomic_formula(1, &[], skel);
        let var_x = builder.typed_variable(10, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_x]);
        let exists_node = builder.exists(exists_vars, a)?;
        let root = builder.not(exists_node);

        // 2. Transformation: ¬∃x.A -> ∀x.¬A
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // L'Exists sous négation doit être devenu un Forall
        assert!(matches!(root_node.kind(), ExprEntryKind::Forall(_)));

        // Vérification du corps : ¬A
        let body_id = root_node.children()[0];
        let body_node = builder.fetch(body_id)?;
        assert!(matches!(body_node.kind(), ExprEntryKind::Not));

        let inner_atom_id = body_node.children()[0];
        assert_eq!(inner_atom_id, a, "L'atome interne a été perdu ou modifié");

        Ok(())
    }

    /// Test that no transformation occurs for a NOT whose child is an atomic formula.
    /// Input: (not (A)) -> (not (A)) (unchanged)
    #[test]
    fn test_push_negation_no_change() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬A (Atome déjà négatif)
        let a = builder.atomic_formula(1, &[], skel);
        let root = builder.not(a);

        // 2. Transformation: Aucun changement structurel attendu
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // Le Kind doit rester Not
        assert!(matches!(root_node.kind(), ExprEntryKind::Not));

        // L'enfant doit toujours être l'atome d'origine
        let child_id = root_node.children()[0];
        assert_eq!(child_id, a, "L'atome interne ne devrait pas être modifié");

        Ok(())
    }

    /// Test pushing negation through a nested expression.
    /// Input: ¬(A ∧ ¬B ∧ ∃x.C)
    /// Expected: (¬A ∨ B ∨ ∀x.¬C)  <-- Note que ¬¬B est devenu B !
    #[test]
    #[test]
    fn test_push_negation_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∧ ¬B ∧ ∃x.C)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        let not_b = builder.not(b);
        let var_x = builder.typed_variable(10, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_x]);
        let exists_c = builder.exists(exists_vars, c)?;

        let and_node = builder.and(&[a, not_b, exists_c]);
        let root = builder.not(and_node);

        // 2. Transformation
        // La logique interne de push_negation va transformer :
        // ¬(A ∧ ¬B ∧ ∃x.C)  =>  (¬A ∨ B ∨ ∀x.¬C)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        assert!(
            matches!(root_node.kind(), ExprEntryKind::Or),
            "La racine doit être un OR"
        );
        let children = root_node.children();
        assert_eq!(children.len(), 3);

        let mut found_not_a = false;
        let mut found_b_simplified = false;
        let mut found_forall_not_c = false;

        for &child_id in children {
            // Comparaison directe par ID (très performant grâce au Hash-Consing)
            if child_id == b {
                found_b_simplified = true;
                continue;
            }

            let node = builder.fetch(child_id)?;
            match node.kind() {
                // Cas ¬A
                ExprEntryKind::Not if node.children()[0] == a => {
                    found_not_a = true;
                }

                // Cas ∀x.¬C
                ExprEntryKind::Forall(_) => {
                    let body_id = node.children()[0];
                    let body_node = builder.fetch(body_id)?;
                    // On vérifie que le corps du Forall est bien Not(C)
                    if matches!(body_node.kind(), ExprEntryKind::Not)
                        && body_node.children()[0] == c
                    {
                        found_forall_not_c = true;
                    }
                }
                _ => {}
            }
        }

        assert!(found_not_a, "¬A est manquant ou mal formé");
        assert!(
            found_b_simplified,
            "B aurait dû être simplifié (¬¬B -> B) et identifié par son ID"
        );
        assert!(found_forall_not_c, "∀x.¬C est manquant ou mal formé");

        Ok(())
    }

    /// Test pushing negation through deep nested structures.
    /// Input: ¬(A ∧ ¬(B ∨ C) ∧ ∀x.∃y.D)
    /// Expected: (¬A ∨ (B ∨ C) ∨ ∃x.∀y.¬D)
    #[test]
    fn test_push_negation_deep_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Définition des atomes et d'une variable pour que les quantificateurs existent
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);
        let d = builder.atomic_formula(4, &[], skel);

        // Ajout d'une variable pour empêcher le builder de supprimer les nœuds
        let var_x = builder.typed_variable(10, &[1]); // ?x de type 1
        let vars = builder.typed_variable_list(vec![var_x]);

        // 2. Construction
        let or_bc = builder.or(&[b, c]);
        let not_or_bc = builder.not(or_bc);

        // ICI : On utilise `vars` (non vide) au lieu de `empty_vars`
        let exists_d = builder.exists(vars.clone(), d)?;
        let forall_exists_d = builder.forall(vars.clone(), exists_d)?;

        let and_node = builder.and(&[a, not_or_bc, forall_exists_d]);
        let root_id = builder.not(and_node);

        // 3. Transformation
        let result_id = to_nnf(root_id, &mut builder, &mut scratch)?;

        // 4. Validation
        let children: Vec<ExprId> = {
            let node = builder.fetch(result_id)?;
            assert!(
                matches!(node.kind(), ExprEntryKind::Or),
                "La racine doit être un OR"
            );
            node.children().to_vec()
        };

        // ¬(A ∧ ¬(B ∨ C) ∧ ∀x.∃x.D)
        // => ¬A ∨ (B ∨ C) ∨ ∃x.¬(∃x.D)
        // => ¬A ∨ B ∨ C ∨ ∃x.∀x.¬D
        assert_eq!(children.len(), 4, "Doit avoir 4 enfants (¬A, B, C, ∃∀¬D)");

        // 5. Vérifications
        let not_a = builder.not(a);
        assert!(children.contains(&not_a));
        assert!(children.contains(&b));
        assert!(children.contains(&c));

        let has_quantifier = children.iter().any(|&id| {
            let node = builder.get(id).unwrap();
            // Le ∀ interne est devenu ∃, et le ∃ interne est devenu ∀
            matches!(node.kind(), ExprEntryKind::Exists(_))
        });
        assert!(
            has_quantifier,
            "La branche quantifiée inversée (Exists) doit être présente"
        );

        Ok(())
    }

    /// Vérifie le partage de structure (DAG) et l'efficacité du Hash-Consing.
    ///
    /// Ce test s'assure que :
    /// 1. L'algorithme `push_negation` ne duplique pas le travail : si une sous-expression
    ///    identique apparaît plusieurs fois, elle doit être représentée par le même `ExprId`.
    /// 2. Le Store réutilise les nœuds déjà existants : transformer une expression puis
    ///    la transformer à nouveau séparément doit retourner le même identifiant unique.
    ///
    /// Logique du test :
    /// - On construit ¬((A ∧ B) ∨ (A ∧ C)).
    /// - Après application de De Morgan, on attend (¬(A ∧ B) ∧ ¬(A ∧ C)).
    /// - On vérifie que l'ID du composant ¬(A ∧ B) au sein du résultat global est
    ///   strictement identique à l'ID obtenu en transformant ¬(A ∧ B) de manière isolée.
    #[test]
    fn test_push_negation_dag_sharing() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        // (A ∧ B)
        let and_ab = builder.and(&[a, b]);
        // (A ∧ B) ∨ (A ∧ C) -- On utilise deux branches différentes pour éviter la simplification X v X
        let and_ac = builder.and(&[a, c]);
        let or_node = builder.or(&[and_ab, and_ac]);

        // ROOT: ¬((A ∧ B) ∨ (A ∧ C))
        let root = builder.not(or_node);

        let result_id = to_nnf(root, &mut builder, &mut scratch)?;

        // On calcule manuellement ¬(A ∧ B) pour vérifier le partage
        let not_and_ab = builder.not(and_ab);
        let expected_part_id = to_nnf(not_and_ab, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;

        // Validation
        // 1. La racine est bien un AND (De Morgan sur le OR)
        assert!(
            matches!(root_node.kind(), ExprEntryKind::And),
            "Doit être un AND"
        );

        // 2. L'un des enfants du résultat doit être EXACTEMENT l'ID de la transformation de la branche AB
        let children = root_node.children();
        assert!(
            children.contains(&expected_part_id),
            "Le store doit partager l'ID de la sous-expression transformée"
        );

        Ok(())
    }
    /// Test d'une triple négation.
    /// ¬¬¬A -> ¬A
    #[test]
    fn test_push_negation_triple() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);

        // Décomposition de ¬¬¬A
        let n1 = builder.not(a);
        let n2 = builder.not(n1);
        let not_3_a = builder.not(n2);

        let result_id = to_nnf(not_3_a, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // Le résultat doit être simplement ¬A (ID de n1)
        assert!(matches!(root_node.kind(), ExprEntryKind::Not));
        assert_eq!(root_node.children()[0], a);
        assert_eq!(
            result_id, n1,
            "La triple négation doit être réduite à une seule"
        );
        Ok(())
    }

    /// Test de stabilité (Idempotence de la fonction).
    /// push_negation(push_negation(X)) == push_negation(X)
    #[test]
    fn test_push_negation_idempotency() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);

        // ¬(A ∧ B)
        let and_node = builder.and(&[a, b]);
        let root = builder.not(and_node);

        let first_pass = to_nnf(root, &mut builder, &mut scratch)?;
        let second_pass = to_nnf(first_pass, &mut builder, &mut scratch)?;

        assert_eq!(
            first_pass, second_pass,
            "Appliquer push_negation deux fois ne doit rien changer"
        );
        Ok(())
    }
}
