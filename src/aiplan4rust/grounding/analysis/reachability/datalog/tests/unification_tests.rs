/*use crate::aiplan4rust::grounding::analysis::reachability::datalog::engine::{
    DatalogEngine, MAX_VARS,
};
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{ObjectId, VariableId};
use crate::analysis::inertia::InertiaTable;

#[test]
fn test_unification_logic() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);

    // On définit une macro locale pour réinitialiser l'environnement.
    // Contrairement à une closure, elle ne crée pas d'emprunt (borrow) persistant.
    macro_rules! reset_env {
        () => {
            engine.current_env.fill(None);
        };
    }

    let var_v0 = Term::Variable(VariableId::from(0));
    let var_v1 = Term::Variable(VariableId::from(1));
    let const_c10 = Term::Constant(ObjectId::from(10));

    // --- Cas 1 : Liaison d'une nouvelle variable ---
    reset_env!();
    let terms = vec![var_v0.clone(), const_c10.clone()];
    let tuple = vec![ObjectId::from(55), ObjectId::from(10)];

    assert!(engine.unify_and_bind(&terms, &tuple));
    assert_eq!(engine.current_env[0], Some(ObjectId::from(55)));

    // --- Cas 2 : Conflit avec une variable déjà liée ---
    // On ne reset pas l'env ici pour tester la persistance de v0 = 55
    let tuple_conflict = vec![ObjectId::from(99), ObjectId::from(10)];
    assert!(
        !engine.unify_and_bind(&terms, &tuple_conflict),
        "Devrait échouer car v0 est déjà lié à 55"
    );

    // --- Cas 3 : Conflit avec une constante ---
    reset_env!();
    let terms_const = vec![const_c10.clone()];
    let tuple_wrong_const = vec![ObjectId::from(11)];
    assert!(
        !engine.unify_and_bind(&terms_const, &tuple_wrong_const),
        "Échec attendu : 10 != 11"
    );

    // --- Cas 4 : Même variable utilisée deux fois (Auto-unification) ---
    reset_env!();
    let terms_double = vec![var_v0.clone(), var_v0.clone()];
    let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
    let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];

    assert!(
        engine.unify_and_bind(&terms_double, &tuple_ok),
        "v0 peut être 7 et 7"
    );

    reset_env!(); // On reset pour repartir à neuf
    assert!(
        !engine.unify_and_bind(&terms_double, &tuple_bad),
        "v0 ne peut pas être 7 ET 8"
    );
}

#[test]
fn test_unification_with_existing_bindings() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    macro_rules! reset_env {
        () => {
            engine.current_env.fill(None);
        };
    }

    let var_r = Term::Variable(VariableId::from(0)); // ?robot
    let var_l = Term::Variable(VariableId::from(1)); // ?loc
    let id_robot1 = ObjectId::from(100);
    let id_room_a = ObjectId::from(1);

    // --- Scénario : Jointure ---
    reset_env!();

    // 1. Première étape : on lie ?robot=100 et ?loc=1
    let terms1 = vec![var_r.clone(), var_l.clone()];
    let tuple1 = vec![id_robot1, id_room_a];
    assert!(engine.unify_and_bind(&terms1, &tuple1));

    // 2. Deuxième étape : on vérifie un autre atome qui réutilise ?robot
    // Si l'atome est Fuel(?robot, 50), on doit valider que ?robot est bien 100
    let id_fuel_50 = ObjectId::from(50);
    let id_fuel_99 = ObjectId::from(99);
    let terms2 = vec![var_r.clone(), Term::Variable(VariableId::from(2))]; // [?robot, ?level]

    // Ce tuple devrait échouer car le robot à l'index 0 est '200', pas '100'
    let tuple_wrong_robot = vec![ObjectId::from(200), id_fuel_50];
    assert!(
        !engine.unify_and_bind(&terms2, &tuple_wrong_robot),
        "Devrait échouer : le robot lié est le 100"
    );

    // Ce tuple devrait réussir et lier ?level (index 2) à 50
    let tuple_ok = vec![id_robot1, id_fuel_50];
    assert!(engine.unify_and_bind(&terms2, &tuple_ok));
    assert_eq!(
        engine.current_env[2],
        Some(id_fuel_50),
        "La variable ?level aurait dû être liée à 50"
    );
}

#[test]
fn test_unification_self_constraint() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    macro_rules! reset_env {
        () => {
            engine.current_env.fill(None);
        };
    }

    // On utilise la même variable deux fois : [?v0, ?v0]
    let var_v0 = Term::Variable(VariableId::from(0));
    let terms = vec![var_v0.clone(), var_v0.clone()];

    // Cas A : Les valeurs sont identiques -> Succès
    reset_env!();
    let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
    assert!(
        engine.unify_and_bind(&terms, &tuple_ok),
        "v0 peut être lié à 7 car 7 == 7"
    );
    assert_eq!(engine.current_env[0], Some(ObjectId::from(7)));

    // Cas B : Les valeurs sont différentes -> Échec
    reset_env!();
    let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];
    assert!(
        !engine.unify_and_bind(&terms, &tuple_bad),
        "Doit échouer car v0 ne peut pas être 7 ET 8 en même temps"
    );
}

#[test]
fn test_unification_rollback_on_failure() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    macro_rules! reset_env {
        () => {
            engine.current_env.fill(None);
        };
    }

    let var_v0 = Term::Variable(VariableId::from(0));
    let var_v1 = Term::Variable(VariableId::from(1));
    let const_99 = Term::Constant(ObjectId::from(99));

    reset_env!();

    // On tente d'unifier [?v0, 99] avec [10, 88]
    // La liaison ?v0 = 10 va réussir, mais la constante 99 != 88 va faire échouer l'atome.
    let terms = vec![var_v0.clone(), const_99];
    let tuple = vec![ObjectId::from(10), ObjectId::from(88)];

    let success = engine.unify_and_bind(&terms, &tuple);

    assert!(!success);
    // CRUCIAL : ?v0 ne doit pas être resté lié à 10 !
    assert_eq!(
        engine.current_env[0], None,
        "L'environnement doit être propre après un échec d'unification"
    );
}

#[test]
fn test_unification_triple_variable_constraint() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    let var_x = Term::Variable(VariableId::from(0));
    // Atome : P(?v0, ?v0, ?v0)
    let atom_terms = vec![var_x.clone(), var_x.clone(), var_x.clone()];

    // Cas échec : les trois ne sont pas identiques
    let tuple_fail = vec![ObjectId::from(10), ObjectId::from(10), ObjectId::from(20)];
    assert!(!engine.unify_and_bind(&atom_terms, &tuple_fail));
    assert_eq!(engine.current_env[0], None, "Doit avoir rollback");

    // Cas succès : les trois sont identiques
    let tuple_success = vec![ObjectId::from(30), ObjectId::from(30), ObjectId::from(30)];
    assert!(engine.unify_and_bind(&atom_terms, &tuple_success));
    assert_eq!(engine.current_env[0], Some(ObjectId::from(30)));
}

#[test]
fn test_unification_partial_rollback() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    let var_x = Term::Variable(VariableId::from(0));
    let var_y = Term::Variable(VariableId::from(1));

    // 1. On simule que ?v0 est déjà lié (par un atome précédent dans une règle)
    engine.current_env[0] = Some(ObjectId::from(50));
    // On ajoute manuellement à la pile de trail pour simuler un état propre
    engine.trailing_indices.push(0);

    // 2. On tente d'unifier un nouvel atome Q(?v0, ?v1) avec un tuple incompatible sur ?v0
    // Atome : Q(?v0, ?v1)  | Tuple : (99, 100) -> Échec car ?v0 est déjà 50
    let atom_terms = vec![var_x, var_y];
    let tuple = vec![ObjectId::from(99), ObjectId::from(100)];

    let savepoint = engine.trailing_indices.len(); // Devrait être 1
    assert!(!engine.unify_and_bind(&atom_terms, &tuple));

    // 3. VERIFICATION :
    // ?v1 ne doit pas être lié (échec de l'atome)
    assert_eq!(engine.current_env[1], None);
    // ?v0 doit être TOUJOURS lié à 50 (il ne doit pas avoir été "rollbacké" par erreur)
    assert_eq!(
        engine.current_env[0],
        Some(ObjectId::from(50)),
        "Le rollback a effacé une variable parente !"
    );
}

#[test]
fn test_unification_empty_atom() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    let atom_terms: Vec<Term> = vec![];
    let tuple: Vec<ObjectId> = vec![];

    // Une unification sans termes est trivialement vraie
    assert!(engine.unify_and_bind(&atom_terms, &tuple));
}

#[test]
fn test_unification_full_cleanup() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    let var_x = Term::Variable(VariableId::from(0));

    // On lie ?v0
    engine.unify_and_bind(&[var_x], &[ObjectId::from(100)]);
    assert!(engine.current_env[0].is_some());

    // On simule le reset de fin de règle que tu as mis dans saturate_semi_naive
    engine.current_env.fill(None);
    engine.trailing_indices.clear();

    assert!(engine.current_env[0].is_none());
    assert!(engine.trailing_indices.is_empty());
}

#[test]
fn test_unification_at_limit_64() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);

    // 1. Création d'un atome avec exactement 64 variables : ?v0, ?v1, ..., ?v63
    let atom_terms: Vec<Term> = (0..MAX_VARS)
        .map(|i| Term::Variable(VariableId::from(i)))
        .collect();

    // 2. Création d'un tuple avec 64 valeurs distinctes
    let tuple: Vec<ObjectId> = (0..MAX_VARS).map(|i| ObjectId::from(i)).collect();

    // 3. L'unification doit réussir sans paniquer (le buffer de 64 est suffisant)
    assert!(engine.unify_and_bind(&atom_terms, &tuple));

    // 4. Vérification de la première, d'une intermédiaire et de la toute dernière liaison
    assert_eq!(engine.current_env[0], Some(ObjectId::from(0)));
    assert_eq!(engine.current_env[32], Some(ObjectId::from(32)));
    assert_eq!(engine.current_env[MAX_VARS - 1], Some(ObjectId::from(63)));

    // 5. Test du rollback complet sur 64 variables
    let split = 0;
    engine.undo_to_savepoint(split);
    assert!(engine.current_env[0].is_none());
    assert!(engine.current_env[MAX_VARS - 1].is_none());
}

#[test]
fn test_unification_cross_variable_consistency() {
    let table = InertiaTable::default();
    let mut engine = DatalogEngine::new(&table);
    let var_x = Term::Variable(VariableId::from(0));
    let var_y = Term::Variable(VariableId::from(1));
    let val_100 = ObjectId::from(100);

    // 1. On lie ?v0 à 100
    assert!(engine.unify_and_bind(&[var_x.clone()], &[val_100]));

    // 2. On essaie d'unifier ?v1 avec la MEME valeur 100 (via un autre atome)
    // Cela doit réussir : deux variables peuvent avoir la même valeur.
    assert!(engine.unify_and_bind(&[var_y.clone()], &[val_100]));

    assert_eq!(engine.current_env[0], Some(val_100));
    assert_eq!(engine.current_env[1], Some(val_100));
}
*/
