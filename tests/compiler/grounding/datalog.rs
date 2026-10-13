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
use std::process::Command;
use test_case::test_case;

pub fn test_datalog_cardinality(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running Datalog Reachability Suite with Dynamic FD Oracle in: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Processing Problem [{}] ... ", oracle_key);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        // 1. Lancement de Fast Downward et extraction systématique des données de l'oracle
        let (exp_f, exp_a, expected_actions, _expected_fluents, fd_available) =
            match run_fast_downward_and_get_content(&domain_path, problem_path) {
                Ok(content) => {
                    let acts = extract_actions_from_content(&content).unwrap_or_default();
                    let fluents = extract_fluents_from_content(&content).unwrap_or_default();
                    (fluents.len(), acts.len(), acts, fluents, true)
                }
                Err(e) => {
                    println!(
                        "\x1b[1;33m[WARNING]\x1b[0m Impossible de lancer Fast Downward: {}",
                        e
                    );
                    (0, 0, Vec::new(), Vec::new(), false)
                }
            };

        // 2. Analyse PDDL et Pipeline Datalog
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

        // --- PIPELINE DE PRÉ-TRAITEMENT ---

        let table = match InertiaTable::build(&lifted_problem) {
            Ok(t) => t,
            Err(e) => {
                println!("\x1b[1;31mFAILED (Inertia Table)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

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

        if let Err(e) = expand_with(&mut lifted_problem, &registry, Some(&evaluator)) {
            println!("\x1b[1;31mFAILED (QNF Expansion)\x1b[0m");
            eprintln!("    Error: {}", e);
            success = false;
            continue;
        }

        let negated_predicates = match to_pnf(&mut lifted_problem) {
            Ok(np) => np,
            Err(e) => {
                println!("\x1b[1;31mFAILED (PNF Transformation)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        let mut datalog = match DatalogEngine::encode(
            &mut lifted_problem,
            &registry,
            &table,
            &negated_predicates,
        ) {
            Ok(engine_configured) => engine_configured,
            Err(e) => {
                println!("\x1b[1;31mFAILED (Datalog Load)\x1b[0m");
                eprintln!("    Error: {}", e);
                success = false;
                continue;
            }
        };

        datalog.run();

        let actual_f = datalog.get_reachable_fluents().len();
        let actual_a = datalog.get_reachable_actions().len();

        // 3. Affichage et comparaison avec l'oracle FD
        println!("\x1b[1;32m[INFO / ORACLE]\x1b[0m");
        println!("    -> FD Oracle  : {} Fluents, {} Actions", exp_f, exp_a);
        println!(
            "    -> Datalog    : {} Fluents, {} Actions",
            actual_f, actual_a
        );

        let mut problem_ok = true;

        if fd_available {
            if actual_a != exp_a {
                println!(
                    "    \x1b[1;31m[MISMATCH] Actions attendues: {}, trouvées par Datalog: {}\x1b[0m",
                    exp_a, actual_a
                );
                problem_ok = false;
            }

            if actual_f != exp_f {
                println!(
                    "    \x1b[1;33m[MISMATCH] Fluents attendus: {}, trouvés par Datalog: {}\x1b[0m",
                    exp_f, actual_f
                );
                problem_ok = false;
            }
        }

        if !problem_ok {
            success = false;
            println!("    \x1b[1;31m[FAILED]\x1b[0m\n");
        } else {
            println!("    \x1b[1;32m[PASSED]\x1b[0m\n");
        }
    }

    success
}

pub fn extract_fluents_from_content(sas_content: &str) -> Result<Vec<String>, String> {
    let mut fluents = Vec::new();
    let mut lines = sas_content.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if trimmed == "begin_variable" {
            // Sauter le nom de la variable SAS+ et l'axiom layer
            let _ = lines.next(); // var name/id
            let _ = lines.next(); // -1 ou axiom layer

            // La ligne suivante indique le nombre de valeurs possibles
            if let Some(count_line) = lines.next() {
                if let Ok(count) = count_line.trim().parse::<usize>() {
                    // Les lignes suivantes sont les valeurs (les fluents STRIPS)
                    for _ in 0..count {
                        if let Some(val_line) = lines.next() {
                            fluents.push(val_line.trim().to_string());
                        }
                    }
                }
            }
        }
        // Le bloc "end_variable" n'a pas besoin d'être intercepté s'il n'y a rien de particulier à faire
    }

    Ok(fluents)
}

pub fn extract_actions_from_content(sas_content: &str) -> Result<Vec<String>, String> {
    let mut actions = Vec::new();
    let mut lines = sas_content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "begin_operator" {
            // La ligne suivante contient le nom de l'action et ses paramètres
            if let Some(action_name) = lines.next() {
                actions.push(action_name.trim().to_string());
            }
        }
    }

    Ok(actions)
}

/// Lance Fast Downward et retourne directement le contenu de output.sas sous forme de String
pub fn run_fast_downward_and_get_content(
    domain_path: &Path,
    problem_path: &Path,
) -> Result<String, String> {
    // Récupère le chemin depuis l'environnement ou utilise le chemin par défaut
    let fd_path_str =
        std::env::var("FAST_DOWNWARD_PATH").unwrap_or_else(|_| "fd/fast-downward.py".to_string());

    let fd_path = Path::new(&fd_path_str);
    if !fd_path.exists() {
        return Err(format!(
            "Fast Downward introuvable au chemin : {}. Définis la variable d'environnement FAST_DOWNWARD_PATH ou place le dossier fd/ au bon endroit.",
            fd_path.display()
        ));
    }

    println!("  [Oracle] Lancement de Fast Downward...");

    let output = Command::new("python3")
        .arg(fd_path)
        .arg("--translate") // <-- Placé ici avant les fichiers PDDL pour éviter l'erreur d'option
        .arg(domain_path)
        .arg(problem_path)
        .output()
        .map_err(|e| format!("Impossible de lancer le processus Fast Downward : {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Fast Downward a échoué avec le code de sortie : {}\nStderr: {}",
            output.status, stderr
        ));
    }

    // Fast Downward écrit par défaut un fichier "output.sas" dans le répertoire courant.
    // On le lit directement en mémoire pour récupérer son contenu en String,
    // puis on le retourne proprement.
    let sas_content = std::fs::read_to_string("output.sas")
        .map_err(|e| format!("Impossible de lire le fichier output.sas généré : {}", e))?;

    Ok(sas_content)
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
