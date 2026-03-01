use std::error::Error;
use std::collections::HashSet;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::engine::DatalogEngine;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{Requirement, Type, TypedSymbol, VariableId, TypedList, AtomSkeletonId, ObjectId, ActionSymbolId, CompareOp, TypeId};
use crate::aiplan4rust::lir::ActionDef;
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


    // 6. Configuration de l'Action MOVE
    // move(?r - object, ?from - location, ?to - location)
    let name_move = problem.interner_mut().intern_symbol("move");
    let action_sym = ActionSymbolId::from(0); // Première action

    let mut params_move = TypedList::new();
    let var_r = VariableId::from(0);
    let var_from = VariableId::from(1);
    let var_to = VariableId::from(2);

    params_move.push(TypedSymbol::new(var_r, Type::primitive(type_obj_id)));
    params_move.push(TypedSymbol::new(var_from, Type::primitive(type_loc_id)));
    params_move.push(TypedSymbol::new(var_to, Type::primitive(type_loc_id)));

    // --- Préconditions de MOVE ---
    let mut b = ExprBuilder::new();

    // On extrait les variables d'abord
    let v_r = b.variable(var_r);
    let v_from1 = b.variable(var_from);
    let p_at = b.atomic_formula_with_skeleton(pred_at_sym, vec![v_r, v_from1], sk_at);

    let v_from2 = b.variable(var_from);
    let v_to1 = b.variable(var_to);
    let p_conn = b.atomic_formula_with_skeleton(pred_conn_sym, vec![v_from2, v_to1], sk_conn);

    let move_precond = b.and(vec![p_at, p_conn]);
    b.set_root(move_precond)?;
    let precond_expr = b.finish();

    // --- Effets de MOVE ---
    let mut b = ExprBuilder::new();

    let v_r_eff = b.variable(var_r);
    let v_from_eff = b.variable(var_from);
    let atom_del = b.atomic_formula_with_skeleton(pred_at_sym, vec![v_r_eff, v_from_eff], sk_at);
    let eff_del = b.not(atom_del);

    let v_r_add = b.variable(var_r);
    let v_to_add = b.variable(var_to);
    let eff_add = b.atomic_formula_with_skeleton(pred_at_sym, vec![v_r_add, v_to_add], sk_at);

    let move_effects = b.and(vec![eff_del, eff_add]);
    b.set_root(move_effects)?;
    let effects_expr = b.finish();

    let action_move = ActionDef::new_snap(action_sym, params_move, precond_expr, effects_expr);
    problem.add_action_def(action_move);

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
    assert_eq!(engine.action_threshold, 6);

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

#[test]
fn test_engine_init_facts_ingestion() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;
    engine.load_problem(&problem)?;

    // Dans le mock : (at robot room_a)
    // at = Fluent ID 0 (fluence_threshold = 2)
    // robot = ObjectId 0, room_a = ObjectId 1
    let sk_at = AtomSkeletonId::from(0);
    let args_at = vec![ObjectId::from(0), ObjectId::from(1)];

    assert!(
        engine.db.contains_delta(sk_at, &args_at),
        "Le fait (at robot room_a) devrait être dans la Database Delta"
    );

    // Dans le mock : (connected room_a room_b)
    // connected = Fluent ID 1
    // room_a = 1, room_b = 2
    let sk_conn = AtomSkeletonId::from(1);
    let args_conn = vec![ObjectId::from(1), ObjectId::from(2)];

    assert!(
        engine.db.contains_delta(sk_conn, &args_conn),
        "Le fait (connected room_a room_b) devrait être dans la Database Delta"
    );

    Ok(())
}

#[test]
fn test_type_inheritance_ingestion() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;
    engine.load_problem(&problem)?;

    // IDs du mock : object=2, location=3
    let sk_obj = AtomSkeletonId::from(2);
    let sk_loc = AtomSkeletonId::from(3);
    let id_room_a = ObjectId::from(1);

    // room_a doit être une location
    assert!(engine.db.contains_delta(sk_loc, &[id_room_a]));

    // room_a doit AUSSI être un object (héritage)
    assert!(engine.db.contains_delta(sk_obj, &[id_room_a]),
            "L'objet room_a devrait hériter du type parent 'object'");

    Ok(())
}

#[test]
fn test_action_rule_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;
    engine.load_problem(&problem)?;

    // L'action 'move' est la première insérée (index 0)
    let move_action_idx = 0;
    let rule = engine.get_rule_for_action(move_action_idx);

    // 1. Vérification du nombre d'atomes
    // On attend 3 Type Guards + 1 atome auxiliaire (qui regroupe 'at' et 'connected')
    let body_len = rule.body().len();
    assert!(body_len >= 4, "La règle devrait avoir au moins 4 atomes (trouvé: {})", body_len);

    // 2. Vérification des Type Guards
    let type_guards_count = rule.body().iter()
        .filter(|a| engine.is_type(a.skeleton_id().as_usize()))
        .count();
    assert_eq!(type_guards_count, 3, "Il devrait y avoir exactement 3 type guards");

    // 3. Vérification de la cohérence des variables (le Robot ?r)
    // Dans move(?r, ?from, ?to), ?r est le premier terme de la tête (index 0)
    let var_r = &rule.head().terms()[0];

    // On vérifie que cette variable ?r est présente dans au moins un atome du corps
    // (soit dans son Type Guard, soit dans l'auxiliaire de précondition)
    let found_r_in_body = rule.body().iter()
        .any(|atom| atom.terms().contains(var_r));

    assert!(found_r_in_body, "La variable ?r de la tête doit être présente dans le corps de la règle");

    // 4. Vérification de l'atome auxiliaire
    let aux_atoms: Vec<_> = rule.body().iter()
        .filter(|a| engine.is_auxiliary(a.skeleton_id().as_usize()))
        .collect();

    assert!(!aux_atoms.is_empty(), "L'encodeur aurait dû générer un atome auxiliaire pour le AND des préconditions");

    Ok(())
}

#[test]
fn test_optimize_body_efficiency() {
    let mut engine = DatalogEngine::new();

    // Configuration des seuils :
    // 0-9: Fluents, 10-19: Types
    engine.fluence_threshold = 10;
    engine.type_threshold = 20;

    let sk_at = AtomSkeletonId::from(1);      // Fluent
    let sk_fuel = AtomSkeletonId::from(2);    // Fluent
    let sk_robot = AtomSkeletonId::from(11);  // Type
    let sk_loc = AtomSkeletonId::from(12);    // Type

    // Simuler des tailles de relations dans la DB
    // On imagine 1000 localisations mais seulement 2 robots
    engine.db.insert_delta_fact(sk_robot, &[ObjectId::from(1)]);
    engine.db.insert_delta_fact(sk_robot, &[ObjectId::from(2)]);
    for i in 0..1000 {
        engine.db.insert_delta_fact(sk_loc, &[ObjectId::from(i)]);
    }
    engine.db.commit_delta();

    // Corps de la règle non optimisé :
    // [ At(?r, ?l), IsLocation(?l), Fuel(?r, ?f), IsRobot(?r) ]
    let var_r = Term::Variable(VariableId::from(0));
    let var_l = Term::Variable(VariableId::from(1));
    let var_f = Term::Variable(VariableId::from(2));

    let mut body = vec![
        Atom::new(sk_at, vec![var_r.clone(), var_l.clone()]),
        Atom::new(sk_loc, vec![var_l.clone()]),
        Atom::new(sk_fuel, vec![var_r.clone(), var_f.clone()]),
        Atom::new(sk_robot, vec![var_r.clone()]),
    ];

    // Exécution de l'optimisation
    engine.optimize_body(&mut body);

    // --- VERIFICATIONS ---

    // 1. Le premier doit être IsRobot (Type + Petite taille)
    assert_eq!(body[0].skeleton_id(), sk_robot, "Le type le plus petit doit être premier");

    // 2. Le deuxième doit être un atome qui utilise ?r (déjà lié)
    // Entre At(?r, ?l) et Fuel(?r, ?f), l'ordre dépendra de rel_size
    // ou de leur position initiale si rel_size est identique (0 ici).
    let second_sk = body[1].skeleton_id();
    assert!(second_sk == sk_at || second_sk == sk_fuel, "Le second doit utiliser la variable ?r liée");

    // 3. IsLocation(?l) ne doit pas être en premier malgré que ce soit un Type,
    // car IsRobot est beaucoup plus petit (2 vs 1000).
    assert_ne!(body[0].skeleton_id(), sk_loc);
}

#[test]
fn test_duplicate_fact_prevention() {
    let mut engine = DatalogEngine::new();
    let sk_id = AtomSkeletonId::from(1);
    let obj_1 = ObjectId::from(1);

    // 1. On simule une variable liée ?x = 1
    engine.current_env[0] = Some(obj_1);

    // 2. On crée une règle dont la tête est P(?x)
    let head = Atom::new(sk_id, vec![Term::Variable(VariableId::from(0))]);
    let rule = Rule::new(head, vec![]);

    // 3. Premier appel : doit ajouter au buffer
    engine.evaluate_head(&rule);
    assert_eq!(engine.discovered_facts.len(), 1);

    // 4. Deuxième appel avec les mêmes données : ne doit RIEN ajouter
    engine.evaluate_head(&rule);
    assert_eq!(engine.discovered_facts.len(), 1, "Le doublon n'a pas été filtré dans discovered_facts");

    // 5. On simule le commit dans la DB
    engine.db.insert_delta_fact(sk_id, &[obj_1]);
    engine.discovered_facts.clear();

    // 6. Troisième appel : ne doit RIEN ajouter car c'est déjà dans la DB
    engine.evaluate_head(&rule);
    assert_eq!(engine.discovered_facts.len(), 0, "Le fait existe déjà dans la DB, il ne doit pas être redécouvert");
}

#[test]
fn test_db_semi_naive_cycle() {
    let mut engine = DatalogEngine::new();
    let sk_id = AtomSkeletonId::from(1);
    let args = [ObjectId::from(10)];

    // Phase 1 : Insertion initiale
    engine.db.insert_delta_fact(sk_id, &args);
    assert!(engine.db.contains_delta(sk_id, &args));
    assert!(!engine.db.contains_stable(sk_id, &args));

    // Phase 2 : Fin de l'itération (Commit)
    engine.db.commit_delta();
    assert!(!engine.db.contains_delta(sk_id, &args));
    assert!(engine.db.contains_stable(sk_id, &args));

    // Phase 3 : Nouveau tour (Préparation)
    // Au début de saturate_semi_naive(), on déplace tout dans le delta pour le premier tour
    engine.db.move_all_to_delta();
    assert!(engine.db.contains_delta(sk_id, &args));
    assert!(!engine.db.contains_stable(sk_id, &args));
}

#[test]
fn test_transitive_closure() {
    let mut engine = DatalogEngine::new();
    engine.fluence_threshold = 1; // 0: AncestorOf
    let sk_anc = AtomSkeletonId::from(0);

    let p_a = ObjectId::from(1); // Grand-père
    let p_b = ObjectId::from(2); // Père
    let p_c = ObjectId::from(3); // Fils

    // Faits initiaux : A est parent de B, B est parent de C
    engine.db.insert_delta_fact(sk_anc, &[p_a, p_b]);
    engine.db.insert_delta_fact(sk_anc, &[p_b, p_c]);
    // On ne fait pas commit_delta ici car engine.run() s'en occupe via move_all_to_delta

    // Règle : Ancestor(?x, ?z) :- Ancestor(?x, ?y), Ancestor(?y, ?z)
    let var_x = Term::Variable(VariableId::from(0));
    let var_y = Term::Variable(VariableId::from(1));
    let var_z = Term::Variable(VariableId::from(2));

    let head = Atom::new(sk_anc, vec![var_x.clone(), var_z.clone()]);
    let body = vec![
        Atom::new(sk_anc, vec![var_x, var_y.clone()]),
        Atom::new(sk_anc, vec![var_y, var_z]),
    ];
    engine.rules.push(Rule::new(head, body));

    engine.run();

    // Doit avoir déduit que A est l'ancêtre de C
    assert!(engine.db.contains_stable(sk_anc, &[p_a, p_c]), "La fermeture transitive a échoué");
}

#[test]
fn test_fixed_point_termination() {
    let mut engine = DatalogEngine::new();
    engine.fluence_threshold = 1;
    let sk_p = AtomSkeletonId::from(0);
    let obj = ObjectId::from(1);

    engine.db.insert_delta_fact(sk_p, &[obj]);

    // Règle récursive simple : P(?x) :- P(?x)
    let var_x = Term::Variable(VariableId::from(0));
    let head = Atom::new(sk_p, vec![var_x.clone()]);
    let body = vec![Atom::new(sk_p, vec![var_x])];
    engine.rules.push(Rule::new(head, body));

    // Si le moteur ne gère pas bien les doublons ou le cycle Delta/Stable,
    // engine.run() ne s'arrêtera jamais ici (Timeout).
    engine.run();

    assert_eq!(engine.db.get_relation_size(sk_p), 1, "Le moteur aurait dû s'arrêter avec un seul fait");
}

#[test]
fn test_full_mock_move_reachability() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;
    engine.load_problem(&problem)?;

    // Avant le run, le robot est en room_a (ObjectId 1)
    let sk_at = AtomSkeletonId::from(0);
    let robot = ObjectId::from(0);
    let room_b = ObjectId::from(2);

    assert!(!engine.db.contains_delta(sk_at, &[robot, room_b]));

    // Lancement de la saturation
    engine.run();

    // Après le run, l'action "Move" a dû être déclenchée
    // et le fait (at robot room_b) doit être dans le Stable
    assert!(
        engine.db.contains_stable(sk_at, &[robot, room_b]),
        "Le robot n'a pas atteint la room_b après saturation"
    );

    Ok(())
}

#[test]
fn test_mixed_arity_zero_and_vars() {
    let mut engine = DatalogEngine::new();
    let sk_prop = AtomSkeletonId::from(0); // Arity 0
    let sk_fact = AtomSkeletonId::from(1); // Arity 1
    let sk_res = AtomSkeletonId::from(2);  // Arity 1
    let obj_a = ObjectId::from(100);
    let obj_b = ObjectId::from(200);

    // Initialement : Prop est FAUX, mais on a des faits
    engine.db.insert_delta_fact(sk_fact, &[obj_a]);
    engine.db.insert_delta_fact(sk_fact, &[obj_b]);

    // Règle : Res(?x) :- Prop(), Fact(?x)
    let var_x = Term::Variable(VariableId::from(0));
    let head = Atom::new(sk_res, vec![var_x.clone()]);
    let body = vec![
        Atom::new(sk_prop, vec![]),
        Atom::new(sk_fact, vec![var_x]),
    ];
    engine.rules.push(Rule::new(head, body));

    // RUN 1 : Ne doit rien produire (Prop est faux)
    engine.run();
    assert_eq!(engine.db.get_relation_size(sk_res), 0);

    // RUN 2 : On active la proposition
    engine.db.insert_delta_fact(sk_prop, &[]);
    engine.run();

    // Doit avoir déduit Res(obj_a) et Res(obj_b)
    assert!(engine.db.contains_stable(sk_res, &[obj_a]));
    assert!(engine.db.contains_stable(sk_res, &[obj_b]));
}

#[test]
fn test_constant_not_in_db() {
    let mut engine = DatalogEngine::new();
    let sk_p = AtomSkeletonId::from(0);
    let c_present = ObjectId::from(1);
    let c_absent = ObjectId::from(999); // N'existe nulle part

    engine.db.insert_delta_fact(sk_p, &[c_present]);

    // Règle : Result(x) :- P(x), P(999)
    // Ne devrait jamais rien produire car P(999) est faux.
    let sk_res = AtomSkeletonId::from(1);
    let var_x = Term::Variable(VariableId::from(0));
    let head = Atom::new(sk_res, vec![var_x.clone()]);
    let body = vec![
        Atom::new(sk_p, vec![var_x]),
        Atom::new(sk_p, vec![Term::Constant(c_absent)]),
    ];
    engine.rules.push(Rule::new(head, body));

    engine.run();

    assert_eq!(engine.db.get_relation_size(sk_res), 0, "Le moteur a déduit un fait alors qu'une constante était absente");
}

#[test]
fn test_triangle_join_consistency() {
    let mut engine = DatalogEngine::new();
    let sk_p = AtomSkeletonId::from(0);
    let (a, b, c) = (ObjectId::from(1), ObjectId::from(2), ObjectId::from(3));

    // Faits : P(a,b), P(b,c), P(c,a) -> Un triangle
    engine.db.insert_delta_fact(sk_p, &[a, b]);
    engine.db.insert_delta_fact(sk_p, &[b, c]);
    engine.db.insert_delta_fact(sk_p, &[c, a]);

    // Règle : Triangle(x,y,z) :- P(x,y), P(y,z), P(z,x)
    let sk_tri = AtomSkeletonId::from(1);
    let (vx, vy, vz) = (Term::Variable(VariableId::from(0)), Term::Variable(VariableId::from(1)), Term::Variable(VariableId::from(2)));
    let head = Atom::new(sk_tri, vec![vx.clone(), vy.clone(), vz.clone()]);
    let body = vec![
        Atom::new(sk_p, vec![vx.clone(), vy.clone()]),
        Atom::new(sk_p, vec![vy, vz.clone()]),
        Atom::new(sk_p, vec![vz, vx]),
    ];
    engine.rules.push(Rule::new(head, body));

    engine.run();

    assert!(engine.db.contains_stable(sk_tri, &[a, b, c]), "Le join cyclique (triangle) a échoué");
}

#[test]
fn test_deep_recursive_chain() {
    let mut engine = DatalogEngine::new();
    let sk_link = AtomSkeletonId::from(0);
    let sk_path = AtomSkeletonId::from(1);
    let sk_goal = AtomSkeletonId::from(2);

    let c1 = ObjectId::from(1);
    let c2 = ObjectId::from(2);
    let c3 = ObjectId::from(3);
    let c4 = ObjectId::from(4);

    // Faits de base : 1->2, 2->3, 3->4
    engine.db.insert_delta_fact(sk_link, &[c1, c2]);
    engine.db.insert_delta_fact(sk_link, &[c2, c3]);
    engine.db.insert_delta_fact(sk_link, &[c3, c4]);

    let vx = Term::Variable(VariableId::from(0));
    let vy = Term::Variable(VariableId::from(1));
    let vz = Term::Variable(VariableId::from(2));

    // Règle 1 : Path(x, y) :- Link(x, y)
    engine.rules.push(Rule::new(
        Atom::new(sk_path, vec![vx.clone(), vy.clone()]),
        vec![Atom::new(sk_link, vec![vx.clone(), vy.clone()])]
    ));

    // Règle 2 : Path(x, z) :- Path(x, y), Link(y, z)
    engine.rules.push(Rule::new(
        Atom::new(sk_path, vec![vx.clone(), vz.clone()]),
        vec![
            Atom::new(sk_path, vec![vx.clone(), vy.clone()]),
            Atom::new(sk_link, vec![vy.clone(), vz.clone()]),
        ]
    ));

    // Règle 3 : Goal(x) :- Path(1, x), Link(x, 4)
    // Cela devrait trouver Goal(3) car Path(1,3) est vrai et Link(3,4) est vrai.
    engine.rules.push(Rule::new(
        Atom::new(sk_goal, vec![vx.clone()]),
        vec![
            Atom::new(sk_path, vec![Term::Constant(c1), vx.clone()]),
            Atom::new(sk_link, vec![vx.clone(), Term::Constant(c4)]),
        ]
    ));

    engine.run();

    // Vérifications
    assert!(engine.db.contains_stable(sk_path, &[c1, c3]), "Le chemin long 1->3 n'a pas été trouvé");
    assert!(engine.db.contains_stable(sk_goal, &[c3]), "Le but final basé sur la récursion a échoué");
}

#[test]
fn test_diamond_join_consistency() {
    let mut engine = DatalogEngine::new();
    let sk_a = AtomSkeletonId::from(0);
    let sk_b = AtomSkeletonId::from(1);
    let sk_c = AtomSkeletonId::from(2);
    let sk_res = AtomSkeletonId::from(3);

    let obj1 = ObjectId::from(1);
    let obj2 = ObjectId::from(2);
    let obj3 = ObjectId::from(3);

    // Règle : Result(x, z) :- A(x, y), B(y, z), C(x, z)
    // C'est un "diamant" : x est lié à y, y à z, et on ferme le tout avec x et z.
    let vx = Term::Variable(VariableId::from(0));
    let vy = Term::Variable(VariableId::from(1));
    let vz = Term::Variable(VariableId::from(2));

    engine.rules.push(Rule::new(
        Atom::new(sk_res, vec![vx.clone(), vz.clone()]),
        vec![
            Atom::new(sk_a, vec![vx.clone(), vy.clone()]),
            Atom::new(sk_b, vec![vy.clone(), vz.clone()]),
            Atom::new(sk_c, vec![vx.clone(), vz.clone()]),
        ]
    ));

    // ÉTAPE 1 : On insère A et B. Rien ne doit se passer (C manque).
    engine.db.insert_delta_fact(sk_a, &[obj1, obj2]);
    engine.db.insert_delta_fact(sk_b, &[obj2, obj3]);
    engine.run();
    assert_eq!(engine.db.get_relation_size(sk_res), 0);

    // ÉTAPE 2 : On insère C. Le diamant est complété.
    engine.db.insert_delta_fact(sk_c, &[obj1, obj3]);
    engine.run();

    // Vérification
    assert!(engine.db.contains_stable(sk_res, &[obj1, obj3]), "Le join en diamant a échoué");
}

#[test]
fn test_engine_execution_with_negated_equality() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // 1. Initialisation (Reset, Fluence Threshold, Encoder)
    engine.load_problem(&problem)?;

    // 2. Récupération des IDs réels du mock
    // Note: Utilise les méthodes de ton LiftedProblem ou du SymbolInterner
    let id_at = 0; // Dans ton mock, 'at' est le premier prédicat ajouté
    let sk_at = AtomSkeletonId::from(0);
    let type_loc_id = 3; // Selon tes tests précédents (fluence_threshold=2 + types)
    let id_robot = ObjectId::from(0);
    let id_room_a = ObjectId::from(1);
    let id_room_b = ObjectId::from(2);

    // 3. Construction de la règle de test via l'ExprBuilder
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    let v0 = builder.variable(VariableId::from(0));
    let v1 = builder.variable(VariableId::from(1));
    let c_robot = builder.constant(id_robot);

    // (AND (at robot ?v0) (NOT (= ?v0 ?v1)))
    // (AND (at robot ?v0) (location ?v1) (NOT (= ?v0 ?v1)))
    let type_loc_id = 3;
    let atom_at = builder.atomic_formula_with_skeleton(id_at, vec![c_robot, v0], sk_at);
    let atom_type_v1 = builder.atomic_formula_with_skeleton(
        type_loc_id,
        vec![v1],
        AtomSkeletonId::from(type_loc_id)
    ); // Borne v1 !
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let not_eq = builder.not(eq);

    let root = builder.and(vec![atom_at, atom_type_v1, not_eq]);
    builder.set_root(root)?;

    // 4. Encodage via l'encodeur de l'engine (qui connaît déjà le threshold)
    let mut params = TypedList::new();
    params.push(TypedSymbol::new(VariableId::from(0), Type::primitive(TypeId::from(type_loc_id))));
    params.push(TypedSymbol::new(VariableId::from(1), Type::primitive(TypeId::from(type_loc_id))));

    let head_atom = engine.encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?.unwrap();
    engine.rules.extend(rules);

    // 5. Injection des faits de test dans la Database
    // On place le robot en Room_A
    engine.db.insert_delta_fact(sk_at, &[id_robot, id_room_a]);

    // On déclare Room_A et Room_B comme étant des locations (TypeId 3)
    let sk_loc = AtomSkeletonId::from(type_loc_id);
    engine.db.insert_delta_fact(sk_loc, &[id_room_a]);
    engine.db.insert_delta_fact(sk_loc, &[id_room_b]);

    // 6. Run !
    engine.run();

    // 7. Vérification finale
    let aux_sk = head_atom.skeleton_id();

    // Le moteur doit trouver (Room_A, Room_B)
    assert!(
        engine.db.contains_stable(aux_sk, &[id_room_a, id_room_b]),
        "L'inégalité v0 != v1 aurait dû permettre de déduire (room_a, room_b)"
    );

    // Le moteur ne doit PAS trouver (Room_A, Room_A)
    assert!(
        !engine.db.contains_stable(aux_sk, &[id_room_a, id_room_a]),
        "L'inégalité v0 != v1 aurait dû bloquer la déduction de (room_a, room_a)"
    );

    Ok(())
}

#[test]
fn test_ground_action_extraction() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // 1. Chargement et exécution
    engine.load_problem(&problem)?;
    engine.run();

    // 2. Extraction des actions que le moteur a jugé "atteignables"
    // Cette méthode utilise ton action_threshold pour filtrer les faits
    let reachable_actions = engine.get_reachable_actions();

    // 3. VÉRIFICATIONS ÉLÉMENTAIRES
    // Dans le mock, on a : (at robot room_a) et (connected room_a room_b)
    // L'action move(robot, room_a, room_b) DOIT être là.
    assert!(!reachable_actions.is_empty(), "Le moteur aurait dû trouver au moins une action valide");

    // 4. VÉRIFICATION PRÉCISE DES PARAMÈTRES
    let id_robot = {
        let sym = problem.interner().lookup_symbol("robot").ok_or("Symbol robot not found")?;
        problem.object_symbol().try_get_id(&sym)? // C'est déjà un Result !
    };

    let id_room_a = {
        let sym = problem.interner().lookup_symbol("room_a").ok_or("Symbol room_a not found")?;
        problem.object_symbol().try_get_id(&sym)?
    };

    let id_room_b = {
        let sym = problem.interner().lookup_symbol("room_b").ok_or("Symbol room_b not found")?;
        problem.object_symbol().try_get_id(&sym)?
    };

    let found_move = reachable_actions.iter().any(|action| {
        // L'ordre des paramètres dans ton mock est : [robot, from, to]
        action.args() == &[id_robot, id_room_a, id_room_b]
    });

    assert!(found_move, "L'action instanciée move(robot, room_a, room_b) est manquante");

    // 5. VÉRIFICATION DU "NOT" (Inégalité)
    let invalid_move = reachable_actions.iter().any(|action| {
        // move(robot, room_a, room_a) ne doit jamais être généré
        action.args() == &[id_robot, id_room_a, id_room_a]
    });

    assert!(!invalid_move, "Le grounder a généré une action move(a, a) malgré l'inégalité");

    Ok(())
}
