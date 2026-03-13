use std::error::Error;
use std::collections::HashSet;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
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

    let loc_def = TypedSymbol::new(type_loc_id, Type::primitive(type_obj_id));
    problem.add_type_defs(loc_def).expect("Failed to add typing def");

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

    // 4. Configuration des Prédicats
    let name_at = problem.interner_mut().intern_symbol("at");
    let pred_at_sym = problem.add_predicate_symbol(name_at);

    let mut params_at = TypedList::new();
    params_at.push(TypedSymbol::new(VariableId::from(0), Type::primitive(type_obj_id)));
    params_at.push(TypedSymbol::new(VariableId::from(1), Type::primitive(type_loc_id)));

    let sk_at = problem.add_predicate_def(AtomicFormulaSkeleton::new(pred_at_sym, params_at));

    let name_conn = problem.interner_mut().intern_symbol("connected");
    let pred_conn_sym = problem.add_predicate_symbol(name_conn);

    let mut params_conn = TypedList::new();
    params_conn.push(TypedSymbol::new(VariableId::from(0), Type::primitive(type_loc_id)));
    params_conn.push(TypedSymbol::new(VariableId::from(1), Type::primitive(type_loc_id)));

    let sk_conn = problem.add_predicate_def(AtomicFormulaSkeleton::new(pred_conn_sym, params_conn));

    // 6. Configuration de l'Action MOVE
    let name_move = problem.interner_mut().intern_symbol("move");

    // --- FIX: Enregistrer le symbole dans le registre du problème ---
    // Cela évite la panique "missing from the registry"
    let action_sym = problem.add_action_symbol(name_move);

    let mut params_move = TypedList::new();
    let var_r = VariableId::from(0);
    let var_from = VariableId::from(1);
    let var_to = VariableId::from(2);

    params_move.push(TypedSymbol::new(var_r, Type::primitive(type_obj_id)));
    params_move.push(TypedSymbol::new(var_from, Type::primitive(type_loc_id)));
    params_move.push(TypedSymbol::new(var_to, Type::primitive(type_loc_id)));

    // --- Préconditions de MOVE ---
    let mut b_pre = ExprBuilder::new();
    let v_r = b_pre.variable(var_r);
    let v_from1 = b_pre.variable(var_from);
    let p_at = b_pre.atomic_formula_with_skeleton(pred_at_sym, vec![v_r, v_from1], sk_at);

    let v_from2 = b_pre.variable(var_from);
    let v_to1 = b_pre.variable(var_to);
    let p_conn = b_pre.atomic_formula_with_skeleton(pred_conn_sym, vec![v_from2, v_to1], sk_conn);

    let move_precond = b_pre.and(vec![p_at, p_conn]);
    b_pre.set_root(move_precond)?;
    let precond_expr = b_pre.finish();

    // --- Effets de MOVE ---
    let mut b_eff = ExprBuilder::new();
    let v_r_eff = b_eff.variable(var_r);
    let v_from_eff = b_eff.variable(var_from);
    let atom_del = b_eff.atomic_formula_with_skeleton(pred_at_sym, vec![v_r_eff, v_from_eff], sk_at);
    let eff_del = b_eff.not(atom_del);

    let v_r_add = b_eff.variable(var_r);
    let v_to_add = b_eff.variable(var_to);
    let eff_add = b_eff.atomic_formula_with_skeleton(pred_at_sym, vec![v_r_add, v_to_add], sk_at);

    let move_effects = b_eff.and(vec![eff_del, eff_add]);
    b_eff.set_root(move_effects)?;
    let effects_expr = b_eff.finish();

    let action_move = ActionDef::new_snap(action_sym, params_move, precond_expr, effects_expr);
    problem.add_action_def(action_move);

    // 7. État Initial (INIT)
    let mut builder = ExprBuilder::new();
    let c_robot = builder.constant(id_robot);
    let c_room_a = builder.constant(id_room_a);
    let c_room_b = builder.constant(id_room_b);

    let fact_at = builder.atomic_formula_with_skeleton(pred_at_sym, vec![c_robot, c_room_a], sk_at);
    let fact_conn = builder.atomic_formula_with_skeleton(pred_conn_sym, vec![c_room_a, c_room_b], sk_conn);

    let root_and = builder.and(vec![fact_at, fact_conn]);
    builder.set_root(root_and)?;
    let init_expr = builder.finish();

    problem.set_init(init_expr);

    Ok(problem)
}

#[test]
fn test_engine_load_segments() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // --- 1. Préparation des négations ---
    // Atome 'at' (ID 0) est marqué comme négatif pour activer le miroir.
    let negated_atoms = vec![AtomSkeletonId::from(0)];

    // Chargement : cela calcule les seuils et compile les règles.
    engine.load_problem(&problem, &negated_atoms)?;

    // --- 2. Vérification des Seuils (Fluents & Miroir) ---
    // Dans le mock : at (0), connected (1) -> fluence_threshold = 2
    assert_eq!(engine.fluence_threshold, 2, "Le seuil des fluents devrait être 2");

    // Miroir activé : type_segment_start = 2 * 2 = 4
    assert_eq!(engine.type_segment_start, 4, "Les types devraient commencer à l'ID 4 (après le miroir)");

    // --- 3. Vérification des Types ---
    // Les types commencent à type_segment_start (4)
    let id_type_object = AtomSkeletonId::from(engine.type_segment_start);
    assert!(engine.is_type(id_type_object), "L'ID {} devrait être un type", id_type_object.as_usize());

    // Le typing ROOT est le dernier du segment des types
    let id_type_root = AtomSkeletonId::from(engine.type_threshold - 1);
    assert!(engine.is_type(id_type_root), "L'ID {} (ROOT) devrait être un type", id_type_root.as_usize());

    // --- 4. Vérification des Actions ---
    // L'action move prend l'ID qui suit les types.
    // L'action_threshold marque la fin de ce segment.
    assert!(engine.action_threshold > engine.type_threshold, "Il devrait y avoir au moins une action");

    // --- 5. Vérification de la Database (Instance des Types) ---
    // Le robot (ObjectId 0) est un 'object' (ID 4)
    let sk_object = AtomSkeletonId::from(engine.type_segment_start);
    let id_robot = ObjectId::from(0);

    assert!(
        engine.db.contains_delta(sk_object, &[id_robot]),
        "Le robot doit être présent dans l'extension du type d'ID {}", sk_object.as_usize()
    );

    // --- 6. Vérification des Auxiliaires ---
    // IMPORTANT : is_auxiliary doit tester par rapport à action_threshold.
    // On teste l'ID qui est PILE au niveau du seuil des actions.
    let first_aux_id = AtomSkeletonId::from(engine.action_threshold);

    assert!(
        engine.is_auxiliary(first_aux_id),
        "L'ID {} (action_threshold) doit être reconnu comme auxiliaire",
        first_aux_id.as_usize()
    );

    // Un ID arbitraire dans la zone de travail
    assert!(engine.is_auxiliary(AtomSkeletonId::from(100)));

    // L'égalité ne doit PAS être un auxiliaire (c'est un builtin)
    let equality_id = AtomSkeletonId::from(Atom::EQUALITY_ID);
    assert!(!engine.is_auxiliary(equality_id), "L'ID d'égalité ne doit pas être un auxiliaire");

    Ok(())
}

#[test]
fn test_engine_init_facts_ingestion() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // On peut passer une liste vide ici si le domaine n'a pas de négations,
    // ou une liste peuplée pour vérifier que les faits POSITIFS ne bougent pas.
    let negated_atoms = Vec::new();
    engine.load_problem(&problem, &negated_atoms)?;

    // --- 1. Vérification du fait : (at robot room_a) ---
    // at = Fluent ID 0
    // robot = ObjectId 0, room_a = ObjectId 1
    let sk_at = AtomSkeletonId::from(0);
    let args_at = vec![ObjectId::from(0), ObjectId::from(1)];

    assert!(
        engine.db.contains_delta(sk_at, &args_at),
        "Le fait (at robot room_a) devrait être dans la Database Delta à l'ID 0"
    );

    // --- 2. Vérification du fait : (connected room_a room_b) ---
    // connected = Fluent ID 1
    // room_a = ObjectId 1, room_b = ObjectId 2
    let sk_conn = AtomSkeletonId::from(1);
    let args_conn = vec![ObjectId::from(1), ObjectId::from(2)];

    assert!(
        engine.db.contains_delta(sk_conn, &args_conn),
        "Le fait (connected room_a room_b) devrait être dans la Database Delta à l'ID 1"
    );

    // --- 3. Vérification de l'étanchéité ---
    // On s'assure qu'aucun fait n'a fuité dans le segment suivant (Types ou Miroir)
    let sk_invalid = AtomSkeletonId::from(engine.type_segment_start);
    assert!(
        !engine.db.contains_delta(sk_invalid, &args_at),
        "Le segment des types ne devrait pas contenir de faits d'atomes"
    );

    Ok(())
}

#[test]
fn test_type_inheritance_ingestion() -> Result<(), Box<dyn Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // On charge le problème. Même avec une liste vide, le moteur
    // initialise correctement type_segment_start.
    engine.load_problem(&problem, &Vec::new())?;

    // --- 1. Récupération dynamique des IDs de types ---
    // Dans ton mock, 'object' est le premier typing (index 0)
    // et 'location' est le second (index 1).
    let sk_obj = AtomSkeletonId::from(engine.type_segment_start);
    let sk_loc = AtomSkeletonId::from(engine.type_segment_start + 1);

    let id_room_a = ObjectId::from(1);

    // --- 2. Vérification de l'appartenance directe ---
    // room_a est défini comme une 'location' dans le mock
    assert!(
        engine.db.contains_delta(sk_loc, &[id_room_a]),
        "L'objet room_a (ID 1) doit être une location (ID {})", sk_loc.as_usize()
    );

    // --- 3. Vérification de l'héritage (Inférence lors du load) ---
    // room_a doit AUSSI être un object car location <: object
    assert!(
        engine.db.contains_delta(sk_obj, &[id_room_a]),
        "L'objet room_a devrait hériter du type parent 'object' (ID {})", sk_obj.as_usize()
    );

    // --- 4. Vérification du typing ROOT ---
    // Tous les objets doivent être dans ROOT (le dernier typing ajouté par le moteur)
    let sk_root = AtomSkeletonId::from(engine.type_threshold - 1);
    assert!(
        engine.db.contains_delta(sk_root, &[id_room_a]),
        "Tout objet doit appartenir au type ROOT (ID {})", sk_root.as_usize()
    );

    Ok(())
}

#[test]
fn test_action_rule_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = DatalogEngine::new();
    let problem = create_mock_problem_with_init()?;

    // On charge le problème. On suppose ici pas de négations pour simplifier les IDs.
    engine.load_problem(&problem, &Vec::new())?;

    // 1. On récupère la règle générée pour l'action 'move' (index 0)
    // Cette règle doit être de la forme : TypeGuards(?r, ?from, ?to), Preconds(...) -> Move(?r, ?from, ?to)
    let move_action_idx = 0;
    let rule = engine.get_rule_for_action(move_action_idx);

    // 2. Vérification des Type Guards
    // Dans le mock, 'move' a 3 paramètres : ?r (robot), ?from (location), ?to (location)
    let type_guards_count = rule.body().iter()
        .filter(|a| engine.is_type(a.skeleton_id()))
        .count();
    assert_eq!(type_guards_count, 3, "Il devrait y avoir exactement 3 type guards pour les paramètres");

    // 3. Vérification de la cohérence des variables (le Robot ?r)
    // On vérifie que la variable en position 0 de l'action est bien liée au corps
    if let Some(var_r) = rule.head().terms().get(0) {
        let found_r_in_body = rule.body().iter()
            .any(|atom| atom.terms().contains(var_r));
        assert!(found_r_in_body, "La variable ?r de la tête doit être présente dans le corps (sécurité de jointure)");
    }

    // 4. Vérification de la logique (Préconditions & Auxiliaires)
    // On filtre tout ce qui n'est pas un typing (donc les fluents PDDL ou les auxiliaires PNF)
    let logical_atoms_count = rule.body().iter()
        .filter(|a| !engine.is_type(a.skeleton_id()))
        .count();

    // On attend au moins 1 atome logique (ex: (at ?r ?from))
    assert!(logical_atoms_count >= 1, "La règle doit contenir au moins une précondition logique");

    // 5. Vérification de l'ID de tête
    // L'ID de l'action doit être dans le segment [type_threshold .. action_threshold]
    let head_id = rule.head().skeleton_id();
    assert!(engine.is_action(head_id), "L'atome de tête doit être identifié comme une Action");

    Ok(())
}

#[test]
fn test_optimize_body_efficiency() {
    let mut engine = DatalogEngine::new();

    // --- CONFIGURATION RÉALISTE DES SEUILS ---
    engine.fluence_threshold = 10;
    engine.type_segment_start = 10;
    engine.type_threshold = 20;

    let sk_at = AtomSkeletonId::from(1);      // Fluent
    let sk_fuel = AtomSkeletonId::from(2);    // Fluent
    let sk_robot = AtomSkeletonId::from(11);  // Type
    let sk_loc = AtomSkeletonId::from(12);    // Type

    // --- REMPLISSAGE DE LA DB ---

    // 1. IsRobot : très petit (2 faits) -> Doit être le gagnant
    engine.db.insert_delta_fact(sk_robot, &[ObjectId::from(1)]);
    engine.db.insert_delta_fact(sk_robot, &[ObjectId::from(2)]);

    // 2. IsLocation : gros (1000 faits)
    for i in 0..1000 {
        engine.db.insert_delta_fact(sk_loc, &[ObjectId::from(i)]);
    }

    // 3. At : gros (1000 faits)
    for i in 0..1000 {
        engine.db.insert_delta_fact(sk_at, &[ObjectId::from(i), ObjectId::from(i+1)]);
    }

    // 4. Fuel : on lui met 10 faits (plus que Robot qui en a 2)
    // C'est ce qui manquait ! Sinon, Fuel (taille 0) passait devant Robot (taille 2)
    for i in 0..10 {
        engine.db.insert_delta_fact(sk_fuel, &[ObjectId::from(1), ObjectId::from(i)]);
    }

    engine.db.commit_delta();

    // --- PRÉPARATION DE LA RÈGLE ---
    let var_r = Term::Variable(VariableId::from(0));
    let var_l = Term::Variable(VariableId::from(1));
    let var_f = Term::Variable(VariableId::from(2));

    let mut body = vec![
        Atom::new(sk_at, vec![var_r.clone(), var_l.clone()]),
        Atom::new(sk_loc, vec![var_l.clone()]),
        Atom::new(sk_fuel, vec![var_r.clone(), var_f.clone()]),
        Atom::new(sk_robot, vec![var_r.clone()]),
    ];

    // Exécution
    engine.optimize_body(&mut body);

    // --- VÉRIFICATIONS ---
    // Maintenant Robot (Taille 2) est plus petit que Fuel (Taille 10), At (1000) et Loc (1000).
    // De plus, c'est un Type (Priorité 0). Il sera 1er.
    assert_eq!(body[0].skeleton_id(), sk_robot, "Le type le plus petit doit être premier");

    // Le deuxième doit être Fuel ou At (car ils utilisent la variable ?r qui vient d'être liée)
    let second_sk = body[1].skeleton_id();
    assert!(second_sk == sk_fuel || second_sk == sk_at, "Le second doit utiliser la variable ?r liée");
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

    // 1. Initialisation avec le pipeline complet
    // On passe une liste vide de négations (ou peuplée si ton mock en utilise)
    engine.load_problem(&problem, &Vec::new())?;

    // 2. État Initial : Le robot est en room_a (ID 1), pas encore en room_b (ID 2)
    let sk_at = AtomSkeletonId::from(0); // Fluent 'at'
    let robot = ObjectId::from(0);
    let room_b = ObjectId::from(2);

    assert!(!engine.db.contains_delta(sk_at, &[robot, room_b]), "Le robot ne devrait pas être en room_b au départ");

    // 3. Exécution du moteur (Saturation Datalog)
    // C'est ici que les règles Preconds -> Action et Action -> Effects s'activent
    engine.run();

    // 4. Vérification de l'Action (Étape intermédiaire cruciale)
    // On vérifie si l'atome d'action 'move' a été déduit pour ces paramètres
    // L'ID de l'action est situé entre type_threshold et action_threshold
    let sk_move = AtomSkeletonId::from(engine.type_threshold);
    let room_a = ObjectId::from(1);
    assert!(
        engine.db.contains_stable(sk_move, &[robot, room_a, room_b]),
        "L'action move(robot, room_a, room_b) aurait dû être déduite"
    );

    // 5. Vérification de l'Effet (Le but final)
    // Le fait (at robot room_b) doit maintenant être dans le Stable
    assert!(
        engine.db.contains_stable(sk_at, &[robot, room_b]),
        "Le robot n'a pas atteint la room_b après saturation des règles"
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
    // On passe une liste vide car l'égalité n'est pas un fluent "miroir", c'est un built-in.
    engine.load_problem(&problem, &Vec::new())?;

    // 2. Récupération des IDs réels du mock via les seuils de l'engine
    let sk_at = AtomSkeletonId::from(0);
    // On récupère dynamiquement l'ID du typing 'location'
    let sk_loc = AtomSkeletonId::from(engine.type_segment_start + 1);
    let type_loc_id = sk_loc.as_usize();

    let id_robot = ObjectId::from(0);
    let id_room_a = ObjectId::from(1);
    let id_room_b = ObjectId::from(2);

    // 3. Construction de la règle de test via l'ExprBuilder
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    let v0 = builder.variable(VariableId::from(0));
    let v1 = builder.variable(VariableId::from(1));
    let c_robot = builder.constant(id_robot);

    // (AND (at robot ?v0) (location ?v1) (NOT (= ?v0 ?v1)))
    let atom_at = builder.atomic_formula_with_skeleton(0, vec![c_robot, v0], sk_at);
    let atom_type_v1 = builder.atomic_formula_with_skeleton(
        type_loc_id,
        vec![v1],
        sk_loc
    );

    // Utilisation de l'opérateur de comparaison standard
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let not_eq = builder.not(eq);

    let root = builder.and(vec![atom_at, atom_type_v1, not_eq]);
    builder.set_root(root)?;

    // 4. Encodage via l'encodeur de l'engine
    let mut params = TypedList::new();
    params.push(TypedSymbol::new(VariableId::from(0), Type::root()));
    params.push(TypedSymbol::new(VariableId::from(1), Type::root()));

    // L'encodeur va transformer le NOT(=) en utilisant Atom::EQUALITY_ID (0xFFFF_FC00)
    let head_atom = engine.encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?.unwrap();
    engine.rules.extend(rules);

    // 5. Injection des faits
    engine.db.insert_delta_fact(sk_at, &[id_robot, id_room_a]);
    engine.db.insert_delta_fact(sk_loc, &[id_room_a]);
    engine.db.insert_delta_fact(sk_loc, &[id_room_b]);

    // 6. Run !
    engine.run();

    // 7. Vérification finale
    let aux_sk = head_atom.skeleton_id();

    // Doit trouver (Room_A, Room_B) car Room_A != Room_B
    assert!(
        engine.db.contains_stable(aux_sk, &[id_room_a, id_room_b]),
        "L'inégalité v0 != v1 aurait dû permettre de déduire (room_a, room_b)"
    );

    // Ne doit PAS trouver (Room_A, Room_A)
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
    // On ajoute simplement l'argument manquant (Vec::new()) pour les négations
    engine.load_problem(&problem, &Vec::new())?;
    engine.run();

    // 2. Extraction des actions que le moteur a jugé "atteignables"
    let reachable_actions = engine.get_reachable_actions();

    // 3. VÉRIFICATIONS ÉLÉMENTAIRES
    assert!(!reachable_actions.is_empty(), "Le moteur aurait dû trouver au moins une action valide");

    // 4. VÉRIFICATION PRÉCISE DES PARAMÈTRES
    let id_robot = {
        let sym = problem.interner().lookup_symbol("robot").ok_or("Symbol robot not found")?;
        problem.object_symbol().try_get_id(&sym)?
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
