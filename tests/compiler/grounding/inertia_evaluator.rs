use std::path::Path;
use test_case::test_case;

// Public API imports from your crate
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::compiler::grounding::binding::evaluator::evaluator::ExprEvaluator;
use aiplan4rust::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use aiplan4rust::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::compiler::lir::expr::{
    Expr, ExprBuilder, ExprId, ExprKind, ExprStore,
};
use aiplan4rust::aiplan4rust::support::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, VariableId,
};

// Assuming these helpers are provided by your pipeline testing framework utilities
use crate::common::compiler::*;
use crate::common::io::*;

// =========================================================================
// IPC 1998 Test Cases
// =========================================================================
#[test_case("tests/fixtures/pddl/ipc98/assembly"; "ipc98_pddl_adl_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/adl"; "ipc98_pddl_adl_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips"; "ipc98_pddl_strips_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/adl"; "ipc98_pddl_adl_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips"; "ipc98_pddl_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/adl"; "ipc98_pddl_adl_movie")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips"; "ipc98_pddl_strips_movie")]
#[test_case("tests/fixtures/pddl/ipc98/mystery-prime/strips"; "ipc98_pddl_strips_mystery_prime")]
#[test_case("tests/fixtures/pddl/ipc98/mystery/strips"; "ipc98_pddl_strips_mystery")]
#[test_case("tests/fixtures/pddl/ipc98/grid/strips"; "ipc98_pddl_strips_grid")]
// =========================================================================
// IPC 2000 Test Cases
// =========================================================================
#[test_case("tests/fixtures/pddl/ipc00/blocks/strips/typed"; "ipc00_pddl_typed_strips_blocks")]
#[test_case("tests/fixtures/pddl/ipc00/blocks/strips/untyped"; "ipc00_pddl_untyped_strips_blocks")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/strips/typed"; "ipc00_pddl_typed_strips_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/strips/untyped"; "ipc00_pddl_untyped_strips_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/adl/full-typed"; "ipc00_pddl_full_typed_adl_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/adl/simple-typed"; "ipc00_pddl_simple_typed_adl_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/freecell/strips/typed"; "ipc00_pddl_typed_strips_freecell")]
#[test_case("tests/fixtures/pddl/ipc00/freecell/strips/untyped"; "ipc00_pddl_untyped_strips_freecell")]
#[test_case("tests/fixtures/pddl/ipc00/logistics/strips/typed"; "ipc00_pddl_typed_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc00/logistics/strips/untyped"; "ipc00_pddl_untyped_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc00/schedule/adl/typed"; "ipc00_pddl_typed_adl_schedule")]
#[test_case("tests/fixtures/pddl/ipc00/schedule/adl/untyped"; "ipc00_pddl_untyped_adl_schedule")]
// =========================================================================
// IPC 2002 Test Cases (STRIPS & Simple-Time)
// =========================================================================
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/typed"; "ipc02_time_auto_typed_depots")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_adl_satellite")]
// =========================================================================
// IPC 2006 Test Cases
// =========================================================================
#[test_case("tests/fixtures/pddl/ipc06/storage/propositional"; "ipc06_storage_prop")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/strips"; "ipc06_trucks_prop_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/strips"; "ipc06_openstacks_prop_strips")]
pub fn test_inertia_evaluator_integration(domain_path: &str) {
    let path = Path::new(domain_path);
    let result = test_evaluator_robustness(path);
    assert!(
        result,
        "Inertia Evaluator integration oracle verification failed for domain: {}",
        domain_path
    );
}

pub fn test_evaluator_robustness(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;35m>>> Starting Inertia Evaluator Oracle Test: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Processing {}... ", oracle_key);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                println!("\x1b[1;31mFAILED (Parsing)\x1b[0m");
                success = false;
                continue;
            }
        };

        let linking = match link(d_res, p_res, &domain_path, problem_path) {
            Some(l) => l,
            None => {
                println!("\x1b[1;31mFAILED (Linking)\x1b[0m");
                success = false;
                continue;
            }
        };

        let mut lir_result = match encode(linking, &domain_path, problem_path) {
            Some(res) => res,
            None => {
                println!("\x1b[1;31mFAILED (Encoding)\x1b[0m");
                success = false;
                continue;
            }
        };

        // --- CORRECTIF : Au lieu de consommer le problème et d'isoler son store,
        // on extrait le problème MAIS on utilise le store de lir_result qui contient
        // toutes les TypedList enregistrées au préalable ! ---
        let pb = lir_result.take_lifted_problem().expect("No lifted problem");

        let table = analyze_inertia(&pb).expect("Inertia analysis failed");
        let registry = ValueRegistry::build(pb.type_defs().as_slice(), pb.object_defs().as_slice())
            .expect("Registry build failed");

        let init_expr = Expr::new(pb.init(), pb.store());

        let evaluator = InertiaEvaluator::build(
            pb.predicate_defs(),
            pb.function_defs(),
            init_expr,
            &table,
            &registry,
            aiplan4rust::aiplan4rust::compiler::grounding::config::DEFAULT_MAX_ARITY,
            aiplan4rust::aiplan4rust::compiler::grounding::config::DEFAULT_MAX_PROJ,
        )
        .expect("InertiaEvaluator build failed");

        let mut current_ok = true;

        // MODIFICATION : Au lieu d'un `local_store` vide, on récupère le store complet
        // du problème, qui contient l'armoire des TypedList (id: 1, 8, etc.)
        // Si build_test_atom requiert un `&mut ExprStore`, on extrait le store mutable de lir_result
        // ou on clone le store original si take_lifted_problem l'a rendu immuable.
        // Option la plus robuste : Cloner le store du problème pour y ajouter nos atomes de test.
        let mut test_store = pb.store().clone();

        for (idx, skel) in pb.predicate_defs().iter().enumerate() {
            let skel_id = AtomSkeletonId::from(idx);
            let inertia = table.get_predicate(skel_id).unwrap();
            let parameters = pb
                .store()
                .fetch_typed_list(skel.parameters())
                .expect("Parameters fetch failed");
            let arity = parameters.len();

            for i in 0..10 {
                let mut args_opts = Vec::with_capacity(arity);

                for arg_idx in 0..arity {
                    let param_type = &parameters[arg_idx].ty();
                    let val = pick_obj_by_index(&registry, param_type, i, arg_idx);
                    args_opts.push(val);
                }

                // 1. CORRECTIF : On construit l'atome dans le store cloné qui possède l'historique des types
                let (atom_id, _) =
                    build_test_atom(&mut test_store, skel.predicate_id(), &args_opts, skel_id);

                // 2. CORRECTIF : L'expression pointe vers le store valide
                let test_expr = Expr::new(atom_id, &test_store);
                let res = evaluator
                    .evaluate(test_expr)
                    .expect("Evaluation failed when it should have succeeded");

                // 3. Oracle extraction (reste inchangé sur le store d'origine)
                let atom_exists = is_fact_in_init(
                    pb.init(),
                    pb.store(),
                    skel.predicate_id(),
                    &args_opts,
                    pb.predicate_defs(),
                );

                // 4. Verification pipeline
                match res {
                    Some(ExprConstant::Boolean(val)) => {
                        if val != atom_exists {
                            println!("\n    \x1b[0;31m- ORACLE ERROR: Atom {:?} is {}, but init says {}\x1b[0m",
                                     skel.predicate_id(), val, atom_exists);
                            current_ok = false;
                        }
                    }
                    None => {
                        let is_grounded = args_opts.iter().all(|a| a.is_some());
                        if !inertia.is_fluent() && is_grounded {
                            println!("\n    \x1b[0;31m- INCOMPLETENESS: Static Grounded {:?} returned None\x1b[0m", skel_id);
                            current_ok = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        if current_ok {
            println!("\x1b[1;32mPASSED\x1b[0m");
        } else {
            success = false;
            println!("\x1b[1;31mFAILED\x1b[0m");
        }
    }
    success
}

fn is_fact_in_init(
    init_id: ExprId,
    store: &ExprStore,
    pred_id: PredicateSymbolId,
    target_args: &[Option<ObjectId>],
    predicate_defs: &[aiplan4rust::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton],
) -> bool {
    if let Some(root_node) = store.get(init_id) {
        // L'état initial est un grand bloc 'And'
        if let ExprKind::And = root_node.kind() {
            for &child_id in root_node.children() {
                if let Some(child_node) = store.get(child_id) {
                    match child_node.kind() {
                        // Cas 1 : L'atome est présent positivement à la racine du And
                        ExprKind::AtomicFormula(skel_id) => {
                            if check_atom_matches(
                                *skel_id,
                                child_node.children(),
                                pred_id,
                                target_args,
                                predicate_defs,
                                store,
                            ) {
                                return true;
                            }
                        }
                        // Cas 2 : L'atome est encapsulé dans un 'Not' (Spécifique à l'ADL de Movie)
                        // On n'entre PAS dedans pour renvoyer true, car cela signifie que le fait est FAUX.
                        ExprKind::Not => {
                            // On ignore délibérément pour que la fonction continue à chercher
                            // s'il existe une version positive ailleurs (ou renvoie false par défaut).
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    false
}

// Helper de vérification d'atome inchangé
fn check_atom_matches(
    skel_id: AtomSkeletonId,
    children: &[ExprId],
    pred_id: PredicateSymbolId,
    target_args: &[Option<ObjectId>],
    predicate_defs: &[aiplan4rust::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton],
    store: &ExprStore,
) -> bool {
    if let Some(skel) = predicate_defs.get(skel_id.as_usize()) {
        if skel.predicate_id() == pred_id {
            let arg_children = &children[1..]; // LIR : index 0 = dummy symbol

            if arg_children.len() == target_args.len() {
                return arg_children.iter().enumerate().all(|(i, &child_id)| {
                    match target_args[i] {
                        None => true,
                        Some(required_obj) => {
                            if let Some(child_node) = store.get(child_id) {
                                if let ExprKind::Object(init_obj) = child_node.kind() {
                                    return *init_obj == required_obj;
                                }
                            }
                            false
                        }
                    }
                });
            }
        }
    }
    false
}

fn pick_obj_by_index(
    registry: &ValueRegistry,
    ty: &Type<TypeId>,
    index: usize,
    arg_pos: usize,
) -> Option<ObjectId> {
    let domain = registry.get_type_domain(ty).ok()?;
    if domain.is_empty() {
        return None;
    }
    Some(domain[(index + arg_pos) % domain.len()])
}

/// Constructs a structurally valid atomic formula node conforming to the LIR format:
/// `children[0] = Dummy/Symbol node`, `children[1..] = Arguments`
fn build_test_atom(
    store: &mut ExprStore,
    pred_id: PredicateSymbolId,
    args: &[Option<ObjectId>],
    skel_id: AtomSkeletonId,
) -> (ExprId, ExprId) {
    let mut builder = ExprBuilder::new(store);

    // Create the mandatory LIR identifier node at index 0
    let symbol_node_id = builder.predicate(pred_id);
    let mut child_nodes = vec![symbol_node_id];

    for (i, opt_obj) in args.iter().enumerate() {
        match opt_obj {
            Some(obj) => child_nodes.push(builder.object(*obj)),
            // Fixed: Replaced `i as u32` with `i as usize` to satisfy VariableId::from trait bound
            None => child_nodes.push(builder.variable(VariableId::from(i))),
        }
    }

    let atom_formula_id = builder.intern(ExprKind::AtomicFormula(skel_id), &child_nodes);
    (atom_formula_id, symbol_node_id)
}
