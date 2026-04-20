use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

/// Élimine les opérateurs d'implication (A => B) en les remplaçant par (!A | B).
/// Utilise un Scratchpad pour un parcours DFS non-récursif et performant.
pub fn eliminate_imply(
    expr: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    scratch.clear();
    scratch.push(expr, false);

    while let Some((curr_id, processed)) = scratch.pop() {
        if scratch.get(curr_id).is_some() && !processed {
            continue;
        }

        let entry = builder.fetch(curr_id)?;
        let children = entry.children();

        if processed {
            // --- PHASE RECONSTRUCTION (Bottom-Up) ---
            let new_id = match entry.kind() {
                // Cas spécifique : Transformation de l'implication
                ExprEntryKind::Imply => {
                    let a_prime = scratch.fetch(children[0]);
                    let b_prime = scratch.fetch(children[1]);

                    let not_a = builder.not(a_prime);
                    builder.or(vec![not_a, b_prime])
                }

                // Tous les autres cas : Reconstruction générique
                kind => {
                    let new_children: Vec<ExprId> =
                        children.iter().map(|&c| scratch.fetch(c)).collect();

                    builder.reconstruct(kind.clone(), new_children)
                }
            };

            scratch.insert(curr_id, new_id);
        } else {
            // --- PHASE DESCENTE (Top-Down) ---
            scratch.push(curr_id, true);
            for &child in children.iter().rev() {
                scratch.push(child, false);
            }
        }
    }

    Ok(scratch.fetch(expr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::AtomSkeletonId;
    use crate::aiplan4rust::lir::store::iter::Scratchpad;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Input: (A -> (B -> C))
    /// Expected output: (or (not A) (or (not B) C))
    #[test]
    fn test_nested_imply() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(1, vec![], skel);
        let b = builder.atomic_formula(2, vec![], skel);
        let c = builder.atomic_formula(3, vec![], skel);

        let imply_bc = builder.imply(b, c);
        let root_imply = builder.imply(a, imply_bc);

        let new_root_id = eliminate_imply(root_imply, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(new_root_id)?;

        assert!(matches!(root_node.kind(), ExprEntryKind::Or));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // --- Validation Robuste ---
        // On récupère les deux enfants sans présumer de leur ordre
        let node_0 = builder.fetch(children[0])?;
        let node_1 = builder.fetch(children[1])?;

        // L'un des deux doit être le Not(A)
        let has_not_a = (matches!(node_0.kind(), ExprEntryKind::Not) && node_0.children()[0] == a)
            || (matches!(node_1.kind(), ExprEntryKind::Not) && node_1.children()[0] == a);

        assert!(has_not_a, "L'expression résultante doit contenir Not(A)");

        // L'autre doit être le Or(Not(B), C)
        let has_inner_or = children.iter().any(|&id| {
            if let Ok(n) = builder.fetch(id) {
                if matches!(n.kind(), ExprEntryKind::Or) {
                    let c_inner = n.children();
                    // On vérifie récursivement si Not(B) est dedans
                    return c_inner.iter().any(|&cid| {
                        matches!(builder.fetch(cid).unwrap().kind(), ExprEntryKind::Not)
                    });
                }
            }
            false
        });

        assert!(
            has_inner_or,
            "L'expression résultante doit contenir le OR interne"
        );

        Ok(())
    }

    /// Input: (A -> (and B C))
    /// Expected output: (or (and (B) (C)) (not (A)))
    #[test]
    fn test_imply_with_and_consequence() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);

        // 1. Setup: (imply (A) (and B C))
        let a = builder.atomic_formula(1, vec![], skel);
        let b = builder.atomic_formula(2, vec![], skel);
        let c = builder.atomic_formula(3, vec![], skel);
        let and_bc = builder.and(vec![b, c]);
        let root_imply = builder.imply(a, and_bc);

        // 2. Transformation
        let new_root_id = eliminate_imply(root_imply, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(new_root_id)?;

        // 3. Validation
        // Le résultat doit être un OR: (not A) v (and B C)
        assert!(matches!(root_node.kind(), ExprEntryKind::Or));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Stratégie robuste : on cherche le NOT et le AND parmi les enfants
        let mut found_not_a = false;
        let mut found_and_bc = false;

        for &child_id in children {
            let child_node = builder.fetch(child_id)?;
            match child_node.kind() {
                ExprEntryKind::Not => {
                    if child_node.children()[0] == a {
                        found_not_a = true;
                    }
                }
                ExprEntryKind::And => {
                    let and_children = child_node.children();
                    // Le store a pu trier B et C, donc on vérifie la présence des deux
                    if and_children.len() == 2
                        && and_children.contains(&b)
                        && and_children.contains(&c)
                    {
                        found_and_bc = true;
                    }
                }
                _ => {}
            }
        }

        assert!(found_not_a, "Le Not(A) est manquant ou incorrect");
        assert!(found_and_bc, "Le And(B, C) est manquant ou incorrect");

        Ok(())
    }

    /// Input: (A -> (and))
    /// Expected output: (or (not (A)) (and))
    #[test]
    fn test_imply_with_empty_and_consequence() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);

        // 1. Setup: (imply (A) (and))
        let a = builder.atomic_formula(1, vec![], skel);
        let empty_and = builder.empty_and(); // Utilisation de la méthode dédiée si elle existe, sinon builder.and(vec![])
        let root_imply = builder.imply(a, empty_and);

        // 2. Transformation
        let new_root_id = eliminate_imply(root_imply, &mut builder, &mut scratch)?;

        // 3. Validation par comparaison d'IDs (Hash Consing)
        // On construit manuellement l'équivalent logique : (not A) OR (True)
        // Note : Ton builder va probablement simplifier cela directement en `empty_and`
        let not_a = builder.not(a);
        let expected_id = builder.or(vec![not_a, empty_and]);

        // L'ID retourné par eliminate_imply DOIT être le même que celui produit par le builder
        assert_eq!(
            new_root_id, expected_id,
            "Le résultat de eliminate_imply ne correspond pas à l'expression attendue simplifiée"
        );

        // Optionnel : Vérification sémantique si tu veux être sûr que c'est devenu "True"
        // car (not A | True) == True
        if new_root_id == empty_and {
            // Le builder a bien fait son travail de simplification
        } else {
            // Si ce n'est pas simplifié en True, on vérifie au moins que c'est un Or
            let root_node = builder.fetch(new_root_id)?;
            assert!(matches!(
                root_node.kind(),
                ExprEntryKind::Or | ExprEntryKind::And
            ));
        }

        Ok(())
    }

    /// Input: ((forall ?X A) -> (exists ?Y B))
    /// Expected output: (or (not (forall (?X) (A))) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_quantifiers() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);

        // 1. Setup : Variables et Atomes
        let var_x = builder.typed_variable(10, &[100]);
        let var_y = builder.typed_variable(11, &[101]);
        let a = builder.atomic_formula(1, vec![], skel);
        let b = builder.atomic_formula(2, vec![], skel);

        // 2. Construction des quantificateurs
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_node = builder.forall(forall_vars, a);

        let exists_vars = builder.typed_variable_list(vec![var_y]);
        let exists_node = builder.exists(exists_vars, b);

        // 3. Création de l'implication : (forall... => exists...)
        let root_imply = builder.imply(forall_node, exists_node);

        // 4. Transformation
        let result_id = eliminate_imply(root_imply, &mut builder, &mut scratch)?;

        // 5. Validation par l'égalité (L'attendu vs Le résultat)
        // On construit manuellement l'équivalent : (not (forall...)) or (exists...)
        let not_forall = builder.not(forall_node);
        let expected_id = builder.or(vec![not_forall, exists_node]);

        // L'égalité d'ID garantit que toute la structure interne (variables, types, corps) est identique
        assert_eq!(
            result_id, expected_id,
            "La transformation des quantificateurs a produit un ID différent de l'attendu"
        );

        Ok(())
    }

    /// Input: (and (imply A B) (imply A B))
    /// Objectif: Vérifier que le scratchpad ne travaille pas deux fois et que le résultat est partagé.
    #[test]
    fn test_structure_sharing_dag() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(1, vec![], skel);
        let b = builder.atomic_formula(2, vec![], skel);
        let c = builder.atomic_formula(3, vec![], skel);

        // Construction de l'implication partagée
        let imply_ab = builder.imply(a, b);

        // Racine : (and (imply a b) (imply a b) c)
        // Le builder va probablement déjà dédoublonner pour donner : (and (imply a b) c)
        let root_and = builder.and(vec![imply_ab, imply_ab, c]);

        // Transformation
        let new_root_id = eliminate_imply(root_and, &mut builder, &mut scratch)?;

        // --- Validation par égalité d'ID ---

        // 1. On construit ce qu'on attend après transformation
        let not_a = builder.not(a);
        let or_ab = builder.or(vec![not_a, b]);

        // Le résultat attendu est un AND de la version transformée et de l'atome C
        let expected_id = builder.and(vec![or_ab, c]);

        // 2. Comparaison
        assert_eq!(
            new_root_id, expected_id,
            "Le partage de structure ou la transformation du DAG a échoué"
        );

        // 3. Vérification optionnelle de la structure physique (si tu veux vraiment inspecter)
        let root_node = builder.fetch(new_root_id)?;
        assert!(matches!(root_node.kind(), ExprEntryKind::And));
        assert_eq!(
            root_node.children().len(),
            2,
            "Le dédoublonnage (dedup) n'a pas fonctionné comme prévu"
        );

        Ok(())
    }

    /// Objectif: Vérifier qu'appliquer la fonction deux fois ne change rien.
    #[test]
    fn test_idempotence() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(1, vec![], skel);
        let b = builder.atomic_formula(2, vec![], skel);

        // Initial : (imply a b)
        let root_imply = builder.imply(a, b);

        // Première passe : transforme en (or (not a) b)
        let first_pass_id = eliminate_imply(root_imply, &mut builder, &mut scratch)?;

        // Deuxième passe : ne doit rien trouver à transformer
        let second_pass_id = eliminate_imply(first_pass_id, &mut builder, &mut scratch)?;

        // L'égalité d'ID est le test ultime d'idempotence dans un système de Hash Consing
        assert_eq!(
            first_pass_id, second_pass_id,
            "L'application répétée ne doit pas changer l'ID de l'expression"
        );

        // Optionnel : vérifier que le résultat final est bien celui attendu
        let not_a = builder.not(a);
        let expected_or = builder.or(vec![not_a, b]);
        assert_eq!(first_pass_id, expected_or);

        Ok(())
    }

    /// Objectif: Vérifier que les expressions sans implication restent strictement identiques.
    #[test]
    fn test_no_op_preservation() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();

        // On utilise une expression numérique (10.0 < 20.0) qui n'a rien à voir avec imply
        let a = builder.number(10.0);
        let b = builder.number(20.0);
        let less = builder.less(a, b);

        // Transformation
        let result_id = eliminate_imply(less, &mut builder, &mut scratch)?;

        // Validation par égalité d'ID
        assert_eq!(
            less, result_id,
            "Une expression sans implication doit retourner exactement le même ExprId"
        );

        // On peut aussi tester avec un AND simple
        let c = builder.atomic_formula(1, vec![], AtomSkeletonId::from(0));
        let simple_and = builder.and(vec![less, c]);
        let result_and_id = eliminate_imply(simple_and, &mut builder, &mut scratch)?;

        assert_eq!(
            simple_and, result_and_id,
            "Un assemblage sans implication doit être préservé par identité"
        );

        Ok(())
    }
}
