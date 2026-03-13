use std::path::Path;
use test_case::test_case;

use crate::common::io::*;
use crate::common::pipeline::*;

use aiplan4rust::aiplan4rust::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;
use aiplan4rust::aiplan4rust::grounding::config;
use aiplan4rust::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::lang::{AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, VariableId};
use aiplan4rust::aiplan4rust::lir::expr::Expr;
use aiplan4rust::aiplan4rust::lir::expr::builder::ExprBuilder;
use aiplan4rust::aiplan4rust::lir::expr::ops::StaticEvaluator;

#[test_case("tests/fixtures/pddl/ipc98/assembly"; "eval_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/adl"; "eval_gripper_adl")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips"; "eval_logistics_strips")]
pub fn test_inertia_evaluator_integration(domain_path: &str) {
    let path = Path::new(domain_path);
    let result = test_evaluator_robustness(path);
    assert!(result, "Inertia Evaluator failed for domain: {}", domain_path);
}

pub fn test_evaluator_robustness(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!("\n\x1b[1;35m>>> Starting Inertia Evaluator Integration Test: {}\x1b[0m", domain_dir.display());

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
            _ => { println!("\x1b[1;31mFAILED (Parsing)\x1b[0m"); success = false; continue; }
        };

        let linking = match link(d_res, p_res, &domain_path, problem_path) {
            Some(l) => l,
            None => { println!("\x1b[1;31mFAILED (Linking)\x1b[0m"); success = false; continue; }
        };

        let mut lir_result = match encode(linking, &domain_path, problem_path) {
            Some(res) => res,
            None => { println!("\x1b[1;31mFAILED (Encoding)\x1b[0m"); success = false; continue; }
        };

        let mut pb = lir_result.take_lifted_problem().expect("No lifted problem");

        // 1. Préparation de l'évaluateur
        let table = analyze_inertia(&pb).expect("Inertia analysis failed");

        let registry = ValueRegistry::build(
            pb.type_defs(),
            pb.object_defs(),
        ).expect("Failed to build ValueRegistry");

        let evaluator = InertiaEvaluator::build(
            pb.predicate_defs(),
            pb.function_defs(),
            pb.init(),
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        ).expect("Failed to build InertiaEvaluator");

        let mut current_ok = true;

        // 2. Échantillonnage des prédicats
        for (idx, skel) in pb.predicate_defs().iter().enumerate() {
            let skel_id = AtomSkeletonId::from(idx);
            let inertia = table.get_predicate(skel_id).unwrap();
            let arity = skel.arity();

            for i in 0..5 {
                // On prépare un vecteur d'Options au lieu d'ObjectIds bruts
                let mut args_opts = vec![None; arity];

                if arity > 0 {
                    let type_desc = skel.parameters()[0].clone();
                    // Appel à la nouvelle fonction sécurisée
                    args_opts[0] = pick_obj_by_index(&registry, type_desc.ty(), i);
                }

                let atomic_formula = build_test_expression(skel.predicate_id(), &args_opts, skel_id);
                let root_id = atomic_formula.try_root_id().expect("No root ID");

                let res = evaluator.evaluate(root_id, &atomic_formula);

                // Un test est "grounded" seulement si on a pu remplir tous les arguments avec Some
                let is_grounded = args_opts.iter().all(|a| a.is_some());

                match res {
                    // ERREUR : Un statique totalement instancié doit être résolu (True/False)
                    None if !inertia.is_fluent() && is_grounded => {
                        println!("\n    \x1b[0;31m- Error: Static Grounded {:?} returned None\x1b[0m", skel_id);
                        current_ok = false;
                    },
                    // ERREUR : Un fluent ne doit jamais être simplifié en constante (Some)
                    Some(val) if inertia.is_fluent() => {
                        println!("\n    \x1b[0;31m- Error: Fluent {:?} simplified to Some({:?})\x1b[0m", skel_id, val);
                        current_ok = false;
                    },
                    // Le reste (Statique avec variable qui renvoie None) est un comportement valide.
                    _ => {}
                }
            }
        }

        if current_ok {
            println!("\x1b[1;32mPASSED\x1b[0m");
        } else {
            success = false;
            println!("\x1b[1;31mFAILED (Evaluator)\x1b[0m");
        }
    }
    success
}

fn pick_obj_by_index(registry: &ValueRegistry, ty: &Type<TypeId>, index: usize) -> Option<ObjectId> {
    let domain = registry.get_type_domain(ty);
    let card = domain.len();

    if card == 0 {
        return None;
    }

    // Accès sécurisé : on est sûr que card > 0
    Some(domain[index % card])
}
fn build_test_expression(
    predicate_id: PredicateSymbolId,
    args: &[Option<ObjectId>],
    skel_id: AtomSkeletonId
) -> Expr {
    let mut builder = ExprBuilder::new();
    let mut arg_nodes = Vec::with_capacity(args.len());

    for (i, opt_obj) in args.iter().enumerate() {
        match opt_obj {
            Some(obj) => {
                // On a un objet réel : on crée une Constante
                arg_nodes.push(builder.constant(*obj));
            }
            None => {
                // Pas d'objet (either_type vide) : on crée une Variable pour tester le symbolique
                arg_nodes.push(builder.variable(VariableId::from(i)));
            }
        }
    }

    builder.atomic_formula_with_skeleton(predicate_id, arg_nodes, skel_id);
    builder.finish()
}
