// Imports du framework de test (communs/pipelines)
use crate::common::compiler::{analyze_file, encode, link};
use crate::common::io::*;
use aiplan4rust::aiplan4rust::compiler::grounding::config;
use aiplan4rust::aiplan4rust::compiler::grounding::passes::to_pnf;
use aiplan4rust::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::compiler::lir::expr::Expr;
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;
use aiplan4rust::analysis::inertia::table::InertiaTable;
use aiplan4rust::qnf::problem::expand_with;
use aiplan4rust::DatalogEngine;
use std::path::Path;
use test_case::test_case;

pub fn test_datalog_cardinality(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running Datalog Reachability Suite in: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Processing Oracle Key [{}] ... ", oracle_key);
        let _ = std::io::Write::flush(&mut std::io::stdout());

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

        let mut lifted_problem = lir_result.take_lifted_problem().expect("No lifted problem");

        // --- PIPELINE DE PRÉ-TRAITEMENT ISSU DE TON NOYAU ---

        // 2. ANALYSE D'INERTIE
        let table = match InertiaTable::build(&lifted_problem) {
            Ok(t) => t,
            Err(e) => {
                println!("\x1b[1;31mFAILED (Inertia Table)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        // 3. VALUE REGISTRY CONSTRUCTION
        let registry = match ValueRegistry::build(
            lifted_problem.type_defs().as_slice(),
            lifted_problem.object_defs().as_slice(),
        ) {
            Ok(r) => r,
            Err(e) => {
                println!("\x1b[1;31mFAILED (Value Registry)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        // Extraction propre de init pour l'évaluateur d'inertie
        let init = Expr::new(lifted_problem.init(), lifted_problem.store());
        let evaluator = match InertiaEvaluator::build(
            lifted_problem.predicate_defs(),
            lifted_problem.function_defs(),
            init,
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        ) {
            Ok(ev) => ev,
            Err(e) => {
                println!("\x1b[1;31mFAILED (Inertia Evaluator)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        // 5. QUANTIFIER EXPANSION
        if let Err(e) = expand_with(&mut lifted_problem, &registry, Some(&evaluator)) {
            println!("\x1b[1;31mFAILED (QNF Expansion)\x1b[0m");
            eprintln!("    Error: {}", e);
            success = false;
            continue;
        }

        // 6. PNF
        let negated_predicates = match to_pnf(&mut lifted_problem) {
            Ok(np) => np,
            Err(e) => {
                println!("\x1b[1;31mFAILED (PNF Transformation)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        // 1. Appel direct à DatalogEngine::load (plus besoin de let mut datalog = DatalogEngine::new(); avant)
        let mut datalog = match DatalogEngine::encode(
            &mut lifted_problem,
            &registry,
            &table,
            &negated_predicates,
        ) {
            Ok(engine_configured) => engine_configured, // L'engine est créé ET configuré d'un coup
            Err(e) => {
                println!("\x1b[1;31mFAILED (Datalog Load)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue; // Passe à l'itération suivante de ta boucle
            }
        };

        // 2. Maintenant datalog possède à nouveau la propriété de l'objet,
        // et tu peux appeler .run() sans erreur !
        datalog.run();

        let actual_f = datalog.get_reachable_fluents().len();
        let actual_a = datalog.get_reachable_actions().len();

        // Comparaison avec les Oracles connus
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
                    println!(
                        "    Expected: ({} Fluents, {} Actions) | Got: ({} Fluents, {} Actions)",
                        exp_f, exp_a, actual_f, actual_a
                    );
                    success = false;
                } else {
                    println!("\x1b[1;32m[PASS]\x1b[0m ({}F, {}A)", actual_f, actual_a);
                }
            }
            None => {
                println!(
                    "\x1b[1;34m[NEW ORACLE]\x1b[0m Grounded Layout -> {} Fluents, {} Actions",
                    actual_f, actual_a
                );
            }
        }
    }

    success
}

#[test_case("tests/fixtures/pddl/ipc98/assembly/adl/"; "ipc98_adl_assembly")]
pub fn test_pddl_datalog(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(
        test_datalog_cardinality(path),
        "Datalog integration test failed for path: {}",
        domain_path
    );
}
