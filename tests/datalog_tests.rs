use std::path::Path;
use test_case::test_case;
use aiplan4rust::aiplan4rust::grounding::config;
use aiplan4rust::aiplan4rust::grounding::problem::registry::value::ValueRegistry;

mod common;
use crate::common::io::*;
use crate::common::pipeline::{analyze_file, encode, link};

use aiplan4rust::DatalogEngine;
use aiplan4rust::type_flattening::problem::flatten as flatten_types;
use aiplan4rust::quantifier_expansion::problem::expand_with;
use aiplan4rust::aiplan4rust::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;

pub fn test_datalog_cardinality(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    // Message global pour confirmer que le test tourne
    println!("\n\x1b[1;36m>>> Starting Datalog Cardinality Test in: {}\x1b[0m", domain_dir.display());

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        // Ligne de debug pour voir quel fichier est en cours de traitement
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

        flatten_types(&mut pb).unwrap();
        let table = analyze_inertia(&pb).unwrap();
        let registry = ValueRegistry::build(pb.type_defs(), pb.object_defs(), config::DEFAULT_VALUE_REGISTRY_SIZE).unwrap();
        let evaluator = InertiaEvaluator::build(pb.predicate_defs(), pb.function_defs(), pb.init(), &table, &registry, config::DEFAULT_MAX_ARITY, config::DEFAULT_MAX_PROJ).unwrap();
        expand_with(&mut pb, &registry, Some(&evaluator)).unwrap();

        let mut datalog = DatalogEngine::new();
        if let Err(e) = datalog.load_problem(&pb) {
            println!("\x1b[1;31mFAILED (Datalog Load)\x1b[0m");
            eprintln!("    Error: {}", e);
            success = false; continue;
        }

        datalog.run();

        let actual_f = datalog.get_reachable_fluents().len();
        let actual_a = datalog.get_reachable_actions().len();

        let expected = match oracle_key.as_str() {
            "combinatorial/pb01.pddl" => Some((13, 15)),
            "combinatorial/pb02.pddl" => Some((267, 680)),
            "combinatorial/pb03.pddl" => Some((701, 6020)),
            "combinatorial/pb04.pddl" => Some((2721, 11050)),
            "combinatorial/pb05.pddl" => Some((6636, 18880)),
            "combinatorial/pb06.pddl" => Some((6891, 39280)),
            "assembly/pb01.pddl" => Some((650, 114)),
            _ => None,
        };

        match expected {
            Some((exp_f, exp_a)) => {
                if actual_f != exp_f || actual_a != exp_a {
                    println!("\x1b[1;31m[FAIL]\x1b[0m");
                    println!("    Expected: ({}F, {}A) | Got: ({}F, {}A)", exp_f, exp_a, actual_f, actual_a);
                    success = false;
                } else {
                    println!("\x1b[1;32m[PASS]\x1b[0m ({}F, {}A)", actual_f, actual_a);
                }
            },
            None => {
                println!("\x1b[1;34m[NEW]\x1b[0m Found {}F, {}A", actual_f, actual_a);
            }
        }
    }

    success
}

#[test_case("tests/integration/other/combinatorial/"; "com")]
//#[test_case("tests/integration/pddl/ipc98/assembly/adl/"; "ipc98_pddl_adl_assembly")]
pub fn test_pddl_datalog(domain_path: &str) {
    let _ = env_logger::builder()
        .is_test(true)
        .try_init();
    let path = Path::new(domain_path);
    assert!(test_datalog_cardinality(path));
}
