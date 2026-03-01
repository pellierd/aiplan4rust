use std::error::Error;
use std::collections::HashSet;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::engine::{DatalogEngine, MAX_VARS};
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
// Ajuste ces imports selon tes chemins réels
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Requirement, Type, TypedSymbol, VariableId, TypedList, AtomSkeletonId, ObjectId};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::expr::ExprBuilder;

mod unification_tests;

pub fn create_mock_problem_with_init() -> Result<LiftedProblem, Box<dyn Error>> {
    // 1. Initialisation de base
    let interner = SymbolInterner::new();
    let mut reqs = HashSet::new();
    reqs.insert(Requirement::Typing);
    reqs.insert(Requirement::Strips);

    let mut problem = LiftedProblem::new(interner, reqs);

    // 2. Configuration des Types
    let name_obj = problem.interner_mut().intern_symbol("object");
    let name_loc = problem.interner_mut().intern_symbol("location");

    let type_obj_id = problem.add_type_symbol(name_obj);
    let type_loc_id = problem.add_type_symbol(name_loc);

    // location est un sous-type de object
    let loc_def = TypedSymbol::new(type_loc_id, Type::primitive(type_obj_id));
    problem.add_type_defs(loc_def).expect("Failed to add type def");

    // 3. Configuration des Objets
    let name_robot = problem.interner_mut().intern_symbol("robot");
    let name_room_a = problem.interner_mut().intern_symbol("room_a");
    let name_room_b = problem.interner_mut().intern_symbol("room_b");

    let id_robot = problem.add_object_symbol(name_robot);
    let id_room_a = problem.add_object_symbol(name_room_a);
    let id_room_b = problem.add_object_symbol(name_room_b);

    problem.add_object_def(TypedSymbol::new(id_robot, Type::primitive(type_obj_id)))?;
    problem.add_object_def(TypedSymbol::new(id_room_a, Type::primitive(type_loc_id)))?;
    problem.add_object_def(TypedSymbol::new(id_room_b, Type::primitive(type_loc_id)))?;

    // 4. Configuration des Prédicats (Skeletons)
    // (at ?o - object ?l - location)
    let name_at = problem.interner_mut().intern_symbol("at");
    let pred_at_sym = problem.add_predicate_symbol(name_at);

    let mut params_at = TypedList::new();
    params_at.push(TypedSymbol::new(VariableId::from(0), Type::primitive(type_obj_id)));
    params_at.push(TypedSymbol::new(VariableId::from(1), Type::primitive(type_loc_id)));

    let sk_at = problem.add_predicate_def(AtomicFormulaSkeleton::new(pred_at_sym, params_at));

    // (connected ?l1 - location ?l2 - location)
    let name_conn = problem.interner_mut().intern_symbol("connected");
    let pred_conn_sym = problem.add_predicate_symbol(name_conn);

    let mut params_conn = TypedList::new();
    params_conn.push(TypedSymbol::new(VariableId::from(0), Type::primitive(type_loc_id)));
    params_conn.push(TypedSymbol::new(VariableId::from(1), Type::primitive(type_loc_id)));

    let sk_conn = problem.add_predicate_def(AtomicFormulaSkeleton::new(pred_conn_sym, params_conn));

    // 5. Construction de l'État Initial (INIT) avec ExprBuilder
    let mut builder = ExprBuilder::new();

    // Création des constantes
    let c_robot = builder.constant(id_robot);
    let c_room_a = builder.constant(id_room_a);
    let c_room_b = builder.constant(id_room_b);

    // Construction des faits atomiques
    let fact_at = builder.atomic_formula_with_skeleton(
        pred_at_sym,
        vec![c_robot, c_room_a],
        sk_at
    );

    let fact_conn = builder.atomic_formula_with_skeleton(
        pred_conn_sym,
        vec![c_room_a, c_room_b],
        sk_conn
    );

    // Regroupement (AND)
    let root_and = builder.and(vec![fact_at, fact_conn]);

    // Finalisation
    builder.set_root(root_and)?;
    let init_expr = builder.finish();

    problem.set_init(init_expr);

    Ok(problem)
}

#[test]
fn test_engine_load_segments() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    // On utilise la fonction qui retourne un Result
    let problem = create_mock_problem_with_init()?;

    engine.load_problem(&problem)?;

    // --- 1. Vérification des Seuils (Thresholds) ---
    // On a 2 prédicats dans le mock : at (0) et connected (1)
    assert_eq!(engine.fluence_threshold, 2, "Le seuil des fluents devrait être 2");

    // is_fluent(id) -> id < 2
    assert!(engine.is_fluent(0)); // at
    assert!(engine.is_fluent(1)); // connected
    assert!(!engine.is_fluent(2)); // Ici commence les types

    // --- 2. Vérification des Types ---
    // Les types commencent à l'ID 2 (fluence_threshold)
    // Mock : object (2), location (3), et le moteur ajoute ROOT (4)
    assert!(engine.is_type(2), "L'ID 2 devrait être le type 'object'");
    assert!(engine.is_type(3), "L'ID 3 devrait être le type 'location'");
    assert!(engine.is_type(4), "L'ID 4 devrait être le type 'ROOT'");

    // --- 3. Vérification des Actions ---
    // Les actions commencent après les types (ID 5+)
    // Si tu n'as pas encore ajouté d'actions dans le mock, engine.action_threshold sera 5
    assert_eq!(engine.action_threshold, 5);

    // --- 4. Vérification de la Database (Types) ---
    // On vérifie que les objets du mock sont bien classés
    // robot (ObjectId 0) est un 'object' (Type Datalog 2)
    let sk_object = AtomSkeletonId::from(2);
    let id_robot = ObjectId::from(0);

    assert!(
        engine.db.contains_delta(sk_object, &[id_robot]),
        "Le robot doit être présent dans l'extension du type 'object'"
    );

    // --- 5. Vérification des Auxiliaires ---
    // Les IDs auxiliaires sont générés à la volée pour les préconditions complexes
    // Ils commencent après le dernier ID d'action.
    assert!(engine.is_auxiliary(10));

    Ok(())
}

/*#[test]
fn test_engine_saturation_minimal() {
    let mut engine = DatalogEngine::new();

    // Config manuelle des segments pour le test
    engine.fluence_threshold = 2; // 0: At, 1: Connected
    let sk_at = AtomSkeletonId::from(0);
    let sk_conn = AtomSkeletonId::from(1);

    let robot = ObjectId::from(100);
    let loc_a = ObjectId::from(1);
    let loc_b = ObjectId::from(2);

    // 1. État Initial
    engine.db.insert_stable_fact(sk_at, &[robot, loc_a]);
    engine.db.insert_stable_fact(sk_conn, &[loc_a, loc_b]);

    // 2. Création de la règle : At(?r, ?to) :- At(?r, ?from), Connected(?from, ?to)
    let head = Atom::new(sk_at, vec![Term::Variable(VariableId::from(0)), Term::Variable(VariableId::from(2))]);
    let body = vec![
        Atom::new(sk_at, vec![Term::Variable(VariableId::from(0)), Term::Variable(VariableId::from(1))]),
        Atom::new(sk_conn, vec![Term::Variable(VariableId::from(1)), Term::Variable(VariableId::from(2))]),
    ];
    engine.rules.push(Rule::new(head, body));

    // 3. Exécution
    engine.run();

    // 4. Vérification : Le fait At(Robot, LocB) doit exister
    assert!(engine.db.contains_stable(sk_at, &[robot, loc_b]), "Le robot aurait dû se déplacer vers LocB");
}

#[test]
fn test_body_optimization_order() {
    let mut engine = DatalogEngine::new();
    engine.fluence_threshold = 10;
    engine.type_threshold = 20; // Les IDs 10-19 sont des types

    let type_atom = Atom::new(AtomSkeletonId::from(15), vec![Term::Variable(VariableId::from(0))]);
    let fluent_atom = Atom::new(AtomSkeletonId::from(1), vec![Term::Variable(VariableId::from(0))]);

    let mut body = vec![fluent_atom.clone(), type_atom.clone()];

    engine.optimize_body(&mut body);

    // Le type (ID 15) doit passer en premier (Index 0)
    assert_eq!(body[0].skeleton_id().as_usize(), 15);
}*/
