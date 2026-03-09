use std::collections::HashMap;
use crate::aiplan4rust::grounding::config::{DEFAULT_MAX_ARITY, DEFAULT_MAX_PROJ};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{AtomSkeletonId, ObjectId};
use crate::aiplan4rust::lir::problem::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::analysis::inertia::evaluator::InertiaEvaluator;
use crate::analysis::inertia::InertiaTable;

#[cfg(test)]
impl<'a> InertiaEvaluator<'a> {
    /// Crée un registre mocké.
    /// Note : l'InertiaTable doit être créée à l'extérieur (dans le test)
    /// pour garantir la durée de vie 'a.
    pub fn mock(
        predicate_defs: &[AtomicFormulaSkeleton], // Plus besoin de 'a ici pour ces deux-là
        function_defs: &[AtomicFunctionSkeleton],
        value_registry: &'a ValueRegistry,        // On garde 'a pour les objets externes
        inertia: &'a InertiaTable,
    ) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
            // On convertit les slices en Box possédées
            predicate_defs: predicate_defs.into(),
            function_defs: function_defs.into(),
            value_registry,
            consensus_values: HashMap::new(),
            max_arity: DEFAULT_MAX_ARITY,
            max_proj: DEFAULT_MAX_PROJ,
        }
    }

    pub fn mock_with_config(
        predicate_defs: &[AtomicFormulaSkeleton],
        function_defs: &[AtomicFunctionSkeleton],
        value_registry: &'a ValueRegistry,
        inertia: &'a InertiaTable,
        max_arity: usize,
        max_proj: usize,
    ) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
            predicate_defs: predicate_defs.into(),
            function_defs: function_defs.into(),
            value_registry,
            consensus_values: HashMap::new(),
            max_arity,
            max_proj,
        }
    }

    /// Injecte manuellement des données de comptage pour simuler l'état initial.
    pub fn inject_predicate_count(&mut self, id: usize, mask: u16, args: Vec<ObjectId>, count: usize) {
        self.counting_predicates
            .entry(AtomSkeletonId::from(id))
            .or_default()
            .entry(mask)
            .or_default()
            .insert(args.into_boxed_slice(), count);
    }
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use super::*;
    use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
    use crate::aiplan4rust::lang::{FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ops::StaticValue;
    use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
    use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;

    /// Helper pour créer des définitions de prédicats de test
    pub fn mock_predicate_defs(count: usize) -> Vec<AtomicFormulaSkeleton> {
        (0..count).map(|i| AtomicFormulaSkeleton::new(
            PredicateSymbolId::from(i),
            TypedList::new()
        )).collect()
    }

    /// Helper pour créer des définitions de fonctions de test
    pub fn mock_function_defs(count: usize) -> Vec<AtomicFunctionSkeleton> {
        (0..count).map(|i| AtomicFunctionSkeleton::new(
            FunctionSymbolId::from(i),
            TypedList::new(),
            // On fournit au moins un TypeId pour éviter le panic
            Type::either(vec![TypeId::from(0)])
        )).collect()
    }

    #[test]
    fn test_empty_registry_returns_false_for_positive_inertia() {
        let mut builder = ExprBuilder::new();
        let skel_id_raw = 1;

        let arg1 = builder.constant(10);
        let arg2 = builder.constant(20);

        // Utilisation de la méthode LIR avec skeleton ID
        let atom_node = builder.atomic_formula_with_skeleton(
            0, // PredicateSymbolId
            vec![arg1, arg2],
            skel_id_raw // AtomSkeletonId
        );
        let expr = builder.finish();

        // --- CONTEXTE DE TEST ---
        // On crée 2 définitions pour que l'index [1] soit valide
        let p_defs = mock_predicate_defs(2);
        let f_defs = vec![];
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // On marque le squelette 1 comme Inerte Positif
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::positive());

        // On crée le registre lié à ces données
        let registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer);

        // Selon IPP : Inerte Positif + Absent de l'état initial (N=0) => FALSE
        assert_eq!(
            res.unwrap(),
            Some(false),
            "Un prédicat inerte positif absent de l'état initial doit être simplifié à False"
        );
    }

    #[test]
    fn test_static_function_evaluation() {
        let mut builder = ExprBuilder::new();
        let func_id_raw = 5;
        let skel_id_raw = 5;
        let obj_id = 100;

        let arg = builder.constant(obj_id);

        // Construction du nœud avec le squelette LIR
        let term_node = builder.function_term_with_skeleton(
            func_id_raw,
            vec![arg],
            skel_id_raw
        );
        let expr = builder.finish();

        // --- PRÉPARATION DES DÉPENDANCES (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(0);
        let f_defs = mock_function_defs(6); // Index 5 inclus
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // Configuration de l'inertie sur la table (avant l'emprunt par le registre)
        i_table.insert_function(FunctionSkeletonId::from(skel_id_raw), Inertia::positive());

        // Création du registre avec les références
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Injection manuelle de la valeur : distance(obj100) = 42.0
        let val = StaticValue::Number(OrderedFloat::from(42.0));
        registry.generate_function_masks(
            FunctionSkeletonId::from(skel_id_raw),
            1, // arity
            &[ObjectId::from(obj_id)],
            val
        );

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(term_node, &expr, &mut buffer);

        // Vérification du résultat
        assert_eq!(
            res.unwrap(),
            Some(StaticValue::Number(OrderedFloat::from(42.0))),
            "La fonction statique doit retourner sa valeur initiale enregistrée"
        );
    }

    #[test]
    fn test_positive_inertia_pruning() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // On crée un atome sans arguments (arité 0)
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        // On crée 2 définitions pour que l'index 1 soit valide
        let p_defs = mock_predicate_defs(2);
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // On définit le prédicat comme Inerte Positif
        // (Rappel : Inerte Positif = n'apparaît dans aucun effet positif = ne peut pas être ajouté)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::positive());

        // Création du registre avec les références
        let registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // L'état initial est vide par défaut dans le mock : N(p, a) = 0
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Selon la Définition 6 de Koehler : Si p est positive inertia et N(p, a) = 0, alors FALSE.
        assert_eq!(
            res,
            Some(false),
            "Un prédicat inerte positif absent de l'état initial doit être simplifié à False"
        );
    }

    #[test]
    fn test_negative_inertia_simplification() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // Pour un prédicat d'arité 0, MAX est toujours 1.
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(2);
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // On définit le prédicat comme Inerte Négatif
        // (N'apparaît dans aucun effet négatif = ne peut pas être supprimé)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::negative());

        // Création du registre avec les références
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Pour satisfaire N = MAX, on insère le fait dans l'état initial.
        // Pour l'arité 0, un seul masque d'arguments vides [] suffit.
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 0, &[]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Selon la Définition 6 de Koehler : Si p est negative inertia et N(p, a) = MAX(p, a), alors TRUE.
        assert_eq!(
            res,
            Some(true),
            "Un prédicat inerte négatif dont toutes les instances sont initialement vraies doit être simplifié à True"
        );
    }

    /// Test pour l'isolation des prédicats (vérifie que les squelettes ne se mélangent pas).
    ///
    /// Ce test vérifie que le registre distingue correctement deux prédicats différents (s1 et s2)
    /// même s'ils partagent des arguments identiques.
    /// Selon les principes de Koehler, la simplification doit être locale aux entrées de l'état
    /// initial propres à chaque prédicat (calcul de N).
    #[test]
    fn test_predicate_isolation() {
        let mut builder = ExprBuilder::new();
        let (p1, s1) = (1, 1);
        let (p2, s2) = (2, 2);
        let obj_id = ObjectId::from(10);

        // On crée l'argument une seule fois pour les deux atomes
        let node_arg = builder.constant(obj_id);
        let args = vec![node_arg];

        let node1 = builder.atomic_formula_with_skeleton(p1, args.clone(), s1);
        let node2 = builder.atomic_formula_with_skeleton(p2, args, s2);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(3); // On a besoin d'index jusqu'à 2
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();

        // On définit les deux prédicats comme Inertes Positifs
        i_table.insert_predicate(AtomSkeletonId::from(s1), Inertia::positive());
        i_table.insert_predicate(AtomSkeletonId::from(s2), Inertia::positive());

        // Création du registre avec les références
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // On enregistre uniquement s1(10). s2(10) reste absent (N=0).
        // arity = 1
        registry.generate_predicate_masks(AtomSkeletonId::from(s1), 1, &[obj_id]);

        let mut buffer = ArgumentBuffer::new();

        // Évaluation de s1(10)
        // Comme il est Inerte Positif ET présent dans l'état initial, il est simplifié à True.
        let res1 = registry.evaluate_predicate_internal(node1, &expr, &mut buffer)
            .expect("L'évaluation a échoué pour s1");
        assert_eq!(res1, Some(true), "Le prédicat s1(10) devrait être trouvé et simplifié à True");

        // Évaluation de s2(10)
        // Comme il est Inerte Positif ET absent de l'état initial (N=0), il est simplifié à False.
        let res2 = registry.evaluate_predicate_internal(node2, &expr, &mut buffer)
            .expect("L'évaluation a échoué pour s2");
        assert_eq!(res2, Some(false), "Le prédicat s2(10) devrait être False (Inertie Positive + Absent)");
    }

    #[test]
    fn test_returns_none_on_variable_argument() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_11 = ObjectId::from(11);

        // 1. On peuple le ValueRegistry avec 2 objets (MAX = 2)
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj_10, Type::either(vec![type_id])),
            TypedSymbol::new(obj_11, Type::either(vec![type_id])),
        ]));

        // 2. On définit manuellement le squelette pour inclure le type de l'argument
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de l'expression P(?x)
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![var_node], skel_id);
        let expr = builder.finish();

        let mut i_table = InertiaTable::empty();
        // On met Positive ET Negative pour simuler une constante parfaite
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::positive());
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::negative());

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 4. On injecte SEULEMENT P(10).
        // N = 1 (P(10) est vrai)
        // MAX = 2 (Le domaine de Type 0 contient {10, 11})
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_10]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Comme N(1) != 0 et N(1) != MAX(2), le registre doit répondre "Je ne sais pas"
        assert!(
            res.is_none(),
            "L'évaluation doit être None car l'atome n'est vrai que pour une partie du domaine"
        );
    }

    #[test]
    fn test_negative_inertia_n_equals_max_with_variable() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);

        // IDs des objets pour le test
        let obj_100 = ObjectId::from(100);
        let obj_101 = ObjectId::from(101);

        // 1. Setup du ValueRegistry (Domaine de taille 2)
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj_100, Type::either(vec![type_id])),
            TypedSymbol::new(obj_101, Type::either(vec![type_id])),
        ]));

        // 2. Setup des définitions (Squelette typé pour calculer MAX)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de l'expression P(?x)
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![var_node], skel_id);
        let expr = builder.finish();

        // 4. Configuration de l'Inertie
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::negative());

        // Initialisation du registre
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. REMPLISSAGE VIA LA FONCTION (N=2 pour le masque 0)
        // Cela va automatiquement créer les entrées HashMap avec les bons types (Box<[ObjectId]>, u16, usize)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_100]);
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_101]);

        // 6. Évaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Analyse : N(2) == MAX(2) + Inertie Négative => TRUE
        assert_eq!(
            res,
            Some(true),
            "Si N=MAX pour un inerte négatif, P(?x) doit être simplifié à True"
        );
    }

    #[test]
    fn test_negative_inertia_arity_0_simplification() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // Atome sans arguments
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        let p_defs = mock_predicate_defs(2);
        let f_defs = Vec::new();
        let v_reg = ValueRegistry::empty();
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::negative());

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // N = 1 (pour arité 0, on n'envoie pas d'objets)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 0, &[]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Ici MAX doit être 1 (produit vide), N est 1. Résultat : True.
        assert_eq!(res, Some(true));
    }

    #[test]
    fn test_partial_instantiation_returns_none() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10);
        let obj20 = ObjectId::from(20);
        let obj21 = ObjectId::from(21); // Second objet pour que MAX = 2

        // 1. Setup du ValueRegistry (pour que le type de ?y ait 2 objets)
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj20, Type::either(vec![type_id])),
            TypedSymbol::new(obj21, Type::either(vec![type_id])),
        ]));

        // 2. Setup des définitions (P prend deux arguments de Type 0)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de P(10, ?var1)
        // Note : On utilise variable(1) car c'est le 2ème argument du squelette
        let arg_const = builder.constant(obj10);
        let arg_var = builder.variable(1);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_const, arg_var], skel_id);
        let expr = builder.finish();

        // 4. Setup Inertie (Inerte Positif)
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::positive());

        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. Enregistrement d'un fait : P(10, 20)
        // Cela va incrémenter le masque pour P(10, ?y) à N=1
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj20]);

        // 6. Évaluation
        let mut buffer = ArgumentBuffer::new();
        // On simule que la constante '10' est déjà résolue dans le buffer
        buffer.push(obj10);

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // - Masque calculé pour P(10, ?y) : Le premier argument est fixé, le second est variable.
        // - N(P, [10, ?]) = 1  (car P(10, 20) existe)
        // - MAX(P, [10, ?]) = 2 (car ?y peut être 20 ou 21)
        // - 0 < N < MAX => On ne peut pas simplifier.
        assert!(
            res.is_none(),
            "Une instanciation partielle avec N < MAX doit retourner None"
        );
    }


    #[test]
    fn test_fix_projection_beyond_first_argument() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10); // Robot 1
        let obj51 = ObjectId::from(51); // Zone 51 (Second argument)

        // 1. Setup du ValueRegistry
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj51, Type::either(vec![type_id])),
        ]));

        // 2. Définitions : P(?x, ?y)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Setup Inertie (Inerte Positif pour la Règle 1 : N=0 => FALSE)
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::positive());

        // 4. INITIALISATION DU REGISTRE AVEC MAX_PROJ = 1
        // C'est ici que le test devient intéressant.
        let mut registry = InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 1);

        // 5. ENREGISTREMENT DU FAIT : P(10, 51)
        // On simule l'appel corrigé qui envoie TOUS les arguments
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj51]);

        // --- CAS DE TEST : Évaluer P(?var0, 51) ---
        // On veut savoir si quelqu'un est en Zone 51, mais on ne précise pas qui (?var0).
        // Le filtre max_proj = 1 autorise l'indexation de l'objet 51 (1 seul argument fixe).

        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj51);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        // extract_mask_dynamic va trouver la constante 51 en deuxième position.
        // Masque attendu (Big Endian) : 0b01 (décimal 1)

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // Si la correction ".take(n_proj)" a été faite :
        // - Le registre a reçu [10, 51].
        // - Il a généré le masque 0b01 pour l'objet 51.
        // - N(P, [?, 51]) = 1.
        // - Comme N > 0 et que c'est une inertie positive, ce n'est pas FALSE.
        // - Comme N=1 et MAX=2 (car ?x peut être 10 ou 51), ce n'est pas TRUE non plus.
        // - Résultat attendu : None.

        // Si la correction n'est PAS faite :
        // - Le process_predicate n'a envoyé que [10] (à cause du .take(1)).
        // - Le masque 0b01 (deuxième position) n'a jamais été enregistré.
        // - N(P, [?, 51]) sera 0.
        // - Résultat : Some(false) <--- ERREUR !

        assert!(
            res.is_none(),
            "Le registre devrait trouver l'objet 51 en deuxième position et retourner None (pas False)"
        );
    }

    #[test]
    fn test_projection_full_simplification_to_true() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // On définit deux types distincts pour isoler les domaines
        let type_robot = TypeId::from(0);
        let type_room = TypeId::from(1);

        let obj10 = ObjectId::from(10); // Le seul robot
        let obj51 = ObjectId::from(51); // La salle

        // 1. Setup du ValueRegistry :
        // ?x (type_robot) n'aura qu'un seul objet possible dans son domaine : obj10.
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_robot])),
            TypedSymbol::new(obj51, Type::either(vec![type_room])),
        ]));

        // 2. Définitions : P(?x:robot, ?y:room)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_robot])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_room])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Setup Inertie : NEGATIVE (pour la règle N=MAX => TRUE)
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::negative());

        // 4. Initialisation avec une config permettant de stocker le masque 0b01
        let mut registry = InertiaEvaluator::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 2);

        // 5. Enregistrement du fait initial : P(10, 51)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj51]);

        // --- CAS DE TEST : Évaluer P(?x, 51) ---
        // ?x est une variable (non instanciée), 51 est une constante.
        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj51);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // - extract_mask_dynamic voit que seul le 2ème argument est fixe => masque 0b01.
        // - N = 1 (P(10, 51) est présent).
        // - MAX = domaine du type de la variable ?x (type_robot) = {obj10} => cardinality 1.
        // - N(1) == MAX(1) et Inerte Négatif => TRUE.

        assert_eq!(
            res,
            Some(true),
            "Le registre devrait simplifier à TRUE car l'unique instance possible du domaine est initialement vraie"
        );
    }

    #[test]
    fn test_perfect_constant_missing_is_always_false() {
        let mut builder = ExprBuilder::new();
        let type_id = TypeId::from(0);

        // On définit un ID de squelette unique
        let skel_id_val = 1;
        let skel_id = AtomSkeletonId::from(skel_id_val);

        // 1. Définition des squelettes
        // Le prédicat 1 est lié au squelette 1
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];

        // 2. ValueRegistry : indispensable pour calculate_max_instances
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(ObjectId::from(100), Type::either(vec![type_id])),
        ]));

        // 3. Inertie : On marque explicitement le SQUELETTE 1 comme Inerte Positif
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(skel_id, Inertia::positive());

        // 4. Création du registre
        let f_defs = Vec::new();
        let registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. Construction de l'atome
        // On s'assure que le nœud d'atome porte BIEN le skel_id 1
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(
            1,             // PredicateId
            vec![var_node], // Arguments (?x)
            skel_id_val    // AtomSkeletonId (stocké dans le nœud)
        );
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation failed");

        // ANALYSE :
        // Si is_positive est true et n_p_a est 0 => Some(false)
        assert_eq!(
            res,
            Some(false),
            "Le prédicat devrait être simplifié à FALSE (Inerte Positif + Absent)"
        );
    }

    #[test]
    fn test_arity_zero_flag_behavior() {
        let mut builder = ExprBuilder::new();
        let (p1, s1_val) = (1, 1);
        let (p2, s2_val) = (2, 2);

        let skel1 = AtomSkeletonId::from(s1_val);
        let skel2 = AtomSkeletonId::from(s2_val);

        // 1. Définitions : Squelettes d'arité 0
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p1), TypedList::new()),
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p2), TypedList::new()),
        ];

        // 2. Setup Inertie : Inerte Positif
        let mut i_table = InertiaTable::empty();
        i_table.insert_predicate(skel1, Inertia::positive());
        i_table.insert_predicate(skel2, Inertia::positive());

        // 3. Initialisation du ValueRegistry
        // On utilise la méthode de test pour s'assurer que le vecteur interne
        // est au moins initialisé, évitant le panic si le code cherche un TypeId.
        let value_registry = ValueRegistry::from_objects(TypedList::new());

        let f_defs = Vec::new();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);

        // On enregistre P1 comme présent à l'état initial
        // N(P1, []) = 1
        registry.generate_predicate_masks(skel1, 0, &[]);

        // 4. Construction des atomes LIR (Arité 0)
        let node1 = builder.atomic_formula_with_skeleton(p1, vec![], s1_val);
        let node2 = builder.atomic_formula_with_skeleton(p2, vec![], s2_val);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();

        // 5. Évaluation
        // node1 (s1) : N=1, MAX=1 -> Inerte + Présent = TRUE
        let res1 = registry.evaluate_predicate_internal(node1, &expr, &mut buffer)
            .expect("Evaluation node1 failed");

        // node2 (s2) : N=0 -> Inerte Positif + Absent = FALSE
        let res2 = registry.evaluate_predicate_internal(node2, &expr, &mut buffer)
            .expect("Evaluation node2 failed");

        assert_eq!(res1, Some(true), "Le flag s1 devrait être TRUE (présent + inerte)");
        assert_eq!(res2, Some(false), "Le flag s2 devrait être FALSE (absent + inerte positif)");
    }

    #[test]
    fn test_mask_differentiation_same_object_different_positions() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id_raw) = (1, 1);
        let skel_id = AtomSkeletonId::from(skel_id_raw);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10);
        let obj99 = ObjectId::from(99);

        // 1. Setup des définitions : P(?x:type0, ?y:type0)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];

        // 2. Setup du ValueRegistry (Indispensable pour que le type soit connu)
        let v_reg = ValueRegistry::from_objects(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj99, Type::either(vec![type_id])),
        ]));

        // 3. Initialisation du registre avec max_arity=2 et max_projection=2
        let i_table = InertiaTable::empty();
        let f_defs = Vec::new();
        let mut registry = InertiaEvaluator::mock_with_config(
            &p_defs,
            &f_defs,
            &v_reg,
            &i_table,
            2,
            2
        );

        // 4. On enregistre le fait P(10, 99) à l'état initial
        // Cela va générer, entre autres, le masque 0b10 pour l'objet 10 (position 0)
        registry.generate_predicate_masks(skel_id, 2, &[obj10, obj99]);

        // 5. Cas de test : On évalue l'atome P(?var0, 10)
        // Ici, le premier argument est une variable, le second est la constante 10.
        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj10);
        let node_id = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id_raw);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        let node = expr.try_node(node_id).unwrap();

        // Extraction dynamique du masque pour P(?, 10)
        let mask = registry.extract_mask_dynamic(node, &expr, &mut buffer);

        // ANALYSE :
        // - Argument 0 est Variable -> bit 0 (poids fort) = 0
        // - Argument 1 est Constante -> bit 1 (poids faible) = 1
        // - Masque attendu : 0b01 (1)
        assert_eq!(mask, 1, "Le masque pour le second argument fixe doit être 0b01 (1)");
        assert_eq!(buffer.len(), 1, "Le buffer doit contenir exactement 1 constante (obj10)");
        assert_eq!(buffer[0], obj10);

        // 6. Vérification de la non-collision
        // On cherche dans la table si on a une entrée pour P avec le masque 0b01 et l'objet [10]
        let n = registry.counting_predicates.get(&skel_id)
            .and_then(|m| m.get(&mask))
            .and_then(|e| e.get(&buffer[..]))
            .copied()
            .unwrap_or(0);

        // On ne doit RIEN trouver.
        // Pourquoi ? Parce que l'objet 10 a été enregistré avec le masque 0b10 (position 0).
        // Ici on interroge la position 1.
        assert_eq!(n, 0, "Collision détectée ! L'objet 10 en position 0 ne doit pas être trouvé pour la position 1");
    }

    #[test]
    /// **Objective:** Verify the evaluation of a static (inert) numeric function.
    ///
    /// This test ensures that when a function is marked as positive inertia (its value never changes),
    /// the evaluator correctly retrieves and returns the constant numeric value associated with
    /// grounded (constant) arguments.
    ///
    /// **Input:**
    /// - A function `f` marked as `Inertia::Positive`.
    /// - An initial assignment: `f(obj_10) = 42.5`.
    /// - An expression node representing the grounded call `f(10)`.
    ///
    /// **Expected Output:**
    /// - `Some(StaticValue::Number(42.5))` representing the successfully simplified constant value.
    fn test_evaluate_function_static_numeric() {
        let mut builder = ExprBuilder::new();
        let (func_id_val, skel_id_val) = (1, 1);
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_a = ObjectId::from(10);
        let val = 42.5;

        // 1. Inertia: Mark the function as positive inertia (static) 🧊
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 2. Direct definition of function skeletons 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(func_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup the evaluator and inject initial state: f(obj_10) = 42.5 🔢
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::empty();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);
        registry.generate_function_masks(skel_id, 1, &[obj_a], StaticValue::Number(OrderedFloat::from(val)));

        // 4. Build the LIR expression: f(10)
        let arg = builder.constant(obj_a);
        let node_id = builder.function_term_with_skeleton(func_id_val, vec![arg], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation of the internal logic
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should not fail for valid grounded inputs");

        assert_eq!(res, Some(StaticValue::Number(OrderedFloat(val))));
    }

    #[test]
    fn test_evaluate_function_non_grounded_diverging_values() {
        let mut builder = ExprBuilder::new();
        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_10 = ObjectId::from(10);
        let obj_20 = ObjectId::from(20);

        // 1. Inertia: Mark as static
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 2. Local definitions for the mock 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup evaluator and inject TWO different values
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::empty();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);

        registry.generate_function_masks(skel_id, 1, &[obj_10], StaticValue::Number(OrderedFloat::from(42.5)));
        registry.generate_function_masks(skel_id, 1, &[obj_20], StaticValue::Number(OrderedFloat::from(100.0)));

        // 4. Build expression with a variable: f(?var0)
        let var_node = builder.variable(0);
        let node_id = builder.function_term_with_skeleton(skel_id_val, vec![var_node], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // The result MUST be None because the function is not grounded
        // and values are not unanimous.
        assert_eq!(res, None);
    }

    #[test]
    /// **Objective:** Ensure non-grounded functions are not prematurely simplified.
    ///
    /// **Input:**
    /// - A static function `f`.
    /// - Injected data for a specific instance: `f(obj_10) = 42.5`.
    /// - An expression node with a variable: `f(?var0)`.
    ///
    /// **Expected Output:**
    /// - `None`, verifying the evaluator correctly identifies the expression as non-simplifiable.
    fn test_evaluate_function_non_grounded_returns_none() {
        let mut builder = ExprBuilder::new();
        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_10 = ObjectId::from(10);
        let val = 42.5;

        // 1. Inertia: Mark the function as static
        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // 2. Define the function skeleton 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup evaluator and inject data for ONE specific object
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::empty();
        let mut registry = InertiaEvaluator::mock(&p_defs, &f_defs, &value_registry, &i_table);
        registry.generate_function_masks(skel_id, 1, &[obj_10], StaticValue::Number(OrderedFloat::from(val)));

        // 4. Build expression with a variable: f(?var0)
        let var_node = builder.variable(0);
        let node_id = builder.function_term_with_skeleton(skel_id_val, vec![var_node], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should handle non-grounded nodes gracefully");

        assert_eq!(res, None, "Should not simplify a function call containing variables");
    }
}
