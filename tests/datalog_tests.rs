use std::path::Path;
use test_case::test_case;
use aiplan4rust::aiplan4rust::grounding::config;
use aiplan4rust::aiplan4rust::grounding::problem::registry::value::ValueRegistry;

mod common;
use crate::common::io::*;
use crate::common::pipeline::{analyze_file, encode, link};

// --- CORRECTION ICI ---
// On utilise les alias que tu as créés dans ton lib.rs
use aiplan4rust::DatalogEngine;
use aiplan4rust::type_flattening::problem::flatten as flatten_types;
use aiplan4rust::quantifier_expansion::problem::{expand as expand_quantifiers, expand_with};
use aiplan4rust::aiplan4rust::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::lir::expr::ExprKind::Comparison;
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;

pub fn test_datalog_reachability(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage des fichiers temporaires
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "datalog.diag");

    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    for problem_path in &problems_to_process {
        let domain_path = find_associated_domain(problem_path).unwrap();

        // --- PIPELINE JUSQU'AU LIR ---
        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);
        let linking = link(d_ana.unwrap(), p_ana.unwrap(), &domain_path, problem_path).unwrap();

        let mut lir_result = match encode(linking, &domain_path, problem_path) {
            Some(res) => res,
            None => { success = false; continue; }
        };

        let mut lifted_problem = lir_result.take_lifted_problem().expect("No lifted problem produced");

        // --- PRÉPARATION DU PROBLÈME (Alignée sur fn ground) ---

        // 1. TYPE FLATTENING
        if let Err(e) = flatten_types(&mut lifted_problem) {
            eprintln!("Type flattening failed: {}", e);
            success = false; continue;
        }

        // 3. ANALYSE D'INERTIE & REGISTRES
        let table = match analyze_inertia(&lifted_problem) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Inertia analysis failed: {}", e);
                success = false; continue;
            }
        };

        // --- CONSTRUCTION DU VALUE REGISTRY ---
        // On utilise la nouvelle méthode statique build()
        let registry = match ValueRegistry::build(
            lifted_problem.type_defs(),
            lifted_problem.object_defs(),
            config::DEFAULT_VALUE_REGISTRY_SIZE, // Injection de la config centralisée
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[1;31mValueRegistry build failed\x1b[0m: {:?}", e);
                success = false;
                continue;
            }
        };

        // 4. CONSTRUCTION DE L'INERTIA REGISTRY (L'évaluateur statique)
        // On décompose l'appel pour extraire les définitions et l'état initial
        let evaluator = match InertiaEvaluator::build(
            lifted_problem.predicate_defs(),
            lifted_problem.function_defs(),
            lifted_problem.init(),
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("InertiaRegistry build failed: {:?}", e);
                success = false;
                continue; // Ou handle l'erreur selon la logique de ton test
            }
        };

        let mut pb = lifted_problem.clone();
        // 5. QUANTIFIER EXPANSION (Utilise l'évaluateur pour simplifier l'arbre)
        // On passe &evaluator et Some(&inertia_registry) pour matcher la signature de ground
        if let Err(e) = expand_with(&mut pb, &registry, Some(&evaluator)) {
            eprintln!("Quantifier expansion failed: {}", e);
            success = false; continue;
        }

        // --- TEST DU DATALOG ---
        println!("\x1b[1;35mDatalog Loading:\x1b[0m {}", problem_path.display());

        let mut datalog = DatalogEngine::new();

        // On charge le problème dans le moteur Datalog pour calculer la reachability
        if let Err(e) = datalog.load_problem(&lifted_problem) {
            eprintln!("\x1b[1;31mDatalog Error\x1b[0m for {}: {}", problem_path.display(), e);
            success = false;
        } else {
            println!("\x1b[1;32mDatalog OK\x1b[0m for {}", problem_path.display());
        }

        datalog.run()
    }

    print_test_status(problems_to_process.len(), filter_problem_files(&all_files).len(), domain_dir);
    success
}

#[test_case("tests/integration/hddl/ipc20/total-order/transport"; "ipc20_transport")]
#[test_case("tests/integration/hddl/ipc20/total-order/depots"; "ipc20_depots")]
// ... Ajoute tes autres test_case ici
pub fn test_hddl_datalog(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(test_datalog_reachability(path));
}
