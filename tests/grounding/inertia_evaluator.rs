/*use crate::common::io::*;
use crate::common::pipeline::*;
use aiplan4rust::aiplan4rust::arena::ArenaNode;
use std::path::Path;
use test_case::test_case;

use aiplan4rust::aiplan4rust::grounding::analysis::inertia::new_table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::grounding::binding::evaluator::ExprConstant;
use aiplan4rust::aiplan4rust::grounding::config;
use aiplan4rust::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, VariableId,
};
use aiplan4rust::aiplan4rust::lir::expr::Expr;
// ExprKind est ici
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;

// IPC 1998
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
// IPC 2000
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
// IPC 2002 - Uniquement STRIPS (Logique pure)
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_freecell")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_freecell")]
// IPC 2002 - Uniquement Temporel (Simple-Time) - Logique pure sans numérique
// --- DEPOTS (Temporel) ---
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/typed"; "ipc02_time_auto_typed_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/untyped"; "ipc02_time_auto_untyped_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/typed"; "ipc02_time_hand_typed_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/untyped"; "ipc02_time_hand_untyped_depots")]
// --- DRIVERLOG (Temporel) ---
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/typed"; "ipc02_time_auto_typed_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/untyped"; "ipc02_time_auto_untyped_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/typed"; "ipc02_time_hand_typed_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/untyped"; "ipc02_time_hand_untyped_driverlog")]
// --- ZENOTRAVEL (Temporel) ---
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/typed"; "ipc02_time_auto_typed_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/untyped"; "ipc02_time_auto_untyped_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/typed"; "ipc02_time_hand_typed_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/untyped"; "ipc02_time_hand_untyped_zenotravel")]
// --- SATELLITE (ADL Temporel - Le test de stress) ---
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/typed"; "ipc02_time_auto_typed_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/untyped"; "ipc02_time_untyped_auto_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/typed"; "ipc02_time_hand_typed_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/untyped"; "ipc02_time_hand_untyped_adl_satellite")]
// --- ROVERS (Souvent inclus dans IPC 2002 / 2006 temporel) ---
// Note: Si tu as le dossier rovers dans tes fixtures
#[test_case("tests/fixtures/pddl/ipc02/rovers/simple-time/automatic/typed"; "ipc02_time_auto_typed_rovers")]
// IPC 2006 - ADL & Temporel (Logique pure)

// --- STORAGE ---
#[test_case("tests/fixtures/pddl/ipc06/storage/propositional"; "ipc06_storage_prop")]
#[test_case("tests/fixtures/pddl/ipc06/storage/time"; "ipc06_storage_time")]
// --- TRUCKS (Attention au 's' à Trucks) ---
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/adl"; "ipc06_trucks_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/strips"; "ipc06_trucks_prop_strips")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/adl"; "ipc06_trucks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/strips"; "ipc06_trucks_time_strips")]
// --- OPENSTACKS ---
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/adl"; "ipc06_openstacks_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/strips"; "ipc06_openstacks_prop_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/adl"; "ipc06_openstacks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/strips"; "ipc06_openstacks_time_strips")]
// --- PATHWAYS ---
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/adl"; "ipc06_pathways_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/strips"; "ipc06_pathways_prop_strips")]
// --- ROVERS ---
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/adl"; "ipc06_rovers_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/strips"; "ipc06_rovers_prop_strips")]
// --- TPP (Majuscules selon ton dossier) ---
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/adl"; "ipc06_tpp_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/strips"; "ipc06_tpp_prop_strips")]
// --- PIPESWORLD ---
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/adl"; "ipc06_pipesworld_prop_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/strips"; "ipc06_pipesworld_prop_strips")]
pub fn test_inertia_evaluator_integration(domain_path: &str) {
    let path = Path::new(domain_path);
    let result = test_evaluator_robustness(path);
    assert!(
        result,
        "Inertia Evaluator failed for domain: {}",
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

        let pb = lir_result.take_lifted_problem().expect("No lifted problem");

        let table = analyze_inertia(&pb).expect("Inertia analysis failed");
        let registry = ValueRegistry::build(pb.type_defs().as_slice(), pb.object_defs().as_slice())
            .expect("Registry build failed");

        let init = Expr::new(pb.init(), pb.store());
        let evaluator = InertiaEvaluator::build(
            pb.predicate_defs(),
            pb.function_defs(),
            init,
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        )
        .expect("InertiaEvaluator build failed");

        let mut current_ok = true;

        for (idx, skel) in pb.predicate_defs().iter().enumerate() {
            let skel_id = AtomSkeletonId::from(idx);
            let inertia = table.get_predicate(skel_id).unwrap();
            let arity = skel.arity();

            for i in 0..10 {
                let is_negated = i % 2 == 0;
                let mut args_opts = Vec::with_capacity(arity);

                for arg_idx in 0..arity {
                    let param_type = &skel.parameters()[arg_idx].ty();
                    let val = pick_obj_by_index(&registry, param_type, i, arg_idx);
                    args_opts.push(val);
                }

                // 1. On construit TOUJOURS l'atome (sans le NOT à l'intérieur de build_test_expression)
                let expr = build_test_atom(skel.predicate_id(), &args_opts, skel_id);
                let root_id = expr.try_root_id().expect("Missing RootId");

                // 2. On évalue l'atome
                let res = evaluator.evaluate(root_id, &expr);

                // 3. L'Oracle nous dit si l'atome existe dans l'init
                let atom_exists = is_fact_in_init(pb.init(), skel.predicate_id(), &args_opts);

                // 4. On calcule ce qu'on attend pour l'atome (si c'est statique)
                match res {
                    Some(ExprConstant:::Boolean(val)) => {
                        // L'évaluateur doit être d'accord avec l'existence dans l'init
                        if val != atom_exists {
                            println!("\n    \x1b[0;31m- ORACLE ERROR: Atom {:?} is {}, but init says {}\x1b[0m",
                                     skel.predicate_id(), val, atom_exists);
                            current_ok = false;
                        }
                    }
                    None => {
                        // Si c'est statique et grounded, il n'a pas le droit de renvoyer None
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
    init: &Expr,
    pred_id: PredicateSymbolId,
    target_args: &[Option<ObjectId>],
) -> bool {
    let mut iter = init.preorder().values();

    while let Some(node) = iter.next() {
        if node.kind() == ExprKind::AtomicFormula {
            let predicate_node_id = node.try_child(0).unwrap();
            let predicate_node = init.try_node(predicate_node_id).unwrap();
            let init_pred_id = predicate_node.content().try_predicate_symbol().unwrap();

            if init_pred_id == pred_id {
                let arg_children = &node.children()[1..];

                if arg_children.len() == target_args.len() {
                    // LOGIQUE WILDCARD :
                    // On matche si (l'arg est None) OU (l'objet est identique)
                    let matches = arg_children.iter().enumerate().all(|(i, &child_id)| {
                        match target_args[i] {
                            None => true, // Le joker accepte n'importe quel objet de l'init
                            Some(required_obj) => {
                                let child_node = init.get_node(child_id).unwrap();
                                let init_obj = child_node.content().try_object().unwrap();
                                init_obj == required_obj
                            }
                        }
                    });
                    if matches {
                        return true;
                    }
                }
            }
            iter.skip_subtree();
        } else if matches!(node.kind(), ExprKind::Not | ExprKind::Comparison) {
            iter.skip_subtree();
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

fn build_test_atom(
    pred_id: PredicateSymbolId,
    args: &[Option<ObjectId>],
    skel_id: AtomSkeletonId,
) -> Expr {
    let mut builder = ExprBuilder::new();
    let mut arg_nodes = Vec::with_capacity(args.len());

    for (i, opt_obj) in args.iter().enumerate() {
        match opt_obj {
            Some(obj) => arg_nodes.push(builder.constant(*obj)),
            None => arg_nodes.push(builder.variable(VariableId::from(i))),
        }
    }

    let atom = builder.atomic_formula_with_skeleton(pred_id, arg_nodes, skel_id);
    builder.set_root(atom).unwrap();
    builder.finish()
}
*/
