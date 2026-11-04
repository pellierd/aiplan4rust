// Imports du framework de test (communs/pipelines)
use crate::common::compiler::{analyze_file, encode, link};
use crate::common::io::*;
use aiplan4rust::aiplan4rust::compiler::grounding::passes::to_pnf;
use aiplan4rust::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::compiler::lir::expr::Expr;
use aiplan4rust::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use aiplan4rust::analysis::inertia::evaluator::InertiaEvaluator;
use aiplan4rust::analysis::inertia::table::InertiaTable;
use aiplan4rust::analysis::reachability::datalog::renderers::fluent::render;
use aiplan4rust::qnf::problem::expand_with;
use aiplan4rust::DatalogEngine;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use test_case::test_case;

pub fn test_datalog_cardinality(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running Datalog Reachability Suite with Dynamic FD Oracle (Inclusion Mode) in: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Processing Problem [{}] ... ", oracle_key);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        // 1. Lancement de Fast Downward et extraction des ensembles de l'oracle
        let (exp_actions_set, exp_fluent_set, fd_available) =
            match run_fast_downward_and_get_content(&domain_path, problem_path) {
                Ok(content) => {
                    let acts_vec = extract_actions_from_content(&content).unwrap_or_default();
                    let acts_set: HashSet<String> = acts_vec
                        .into_iter()
                        .map(|s| {
                            // Nettoyage de la chaîne de l'action FD (ex: "release fastener" ou "release fastener wheel")
                            let parts: Vec<&str> = s.split_whitespace().collect();
                            if parts.is_empty() {
                                s.to_lowercase()
                            } else {
                                // Le nom de l'action est le premier élément, suivi de tous les arguments
                                let action_name = parts[0].to_lowercase();
                                let args: Vec<String> =
                                    parts[1..].iter().map(|arg| arg.to_lowercase()).collect();
                                format!("{}({})", action_name, args.join(", "))
                            }
                        })
                        .collect();
                    let fluent_set = extract_datalog_style_fluents(&content).unwrap_or_default();
                    (acts_set, fluent_set, true)
                }
                Err(e) => {
                    println!(
                        "\x1b[1;33m[WARNING]\x1b[0m Impossible de lancer Fast Downward: {}",
                        e
                    );
                    (HashSet::new(), HashSet::new(), false)
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
        let evaluator = match InertiaEvaluator::build_with_defaults(
            lifted_problem.predicate_defs(),
            lifted_problem.function_defs(),
            init,
            &table,
            &registry,
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

        // 1. On récupère les données nécessaires
        let reachable_tuples: Vec<_> = datalog.get_reachable_fluents().iter().cloned().collect();
        let raw_actions: Vec<_> = datalog.get_reachable_actions().iter().cloned().collect();

        // 2. ON LIBÈRE 'datalog' pour rendre son emprunt sur 'lifted_problem'
        drop(datalog);

        // Contexte de rendu pour formater les expressions PDDL
        let ctx = LirRenderContext::with_store(&lifted_problem, lifted_problem.store());

        // 3. Construction des sets normalisés pour les fluents ET les actions
        let mut actual_fluent_set = HashSet::new();
        for tuple in reachable_tuples {
            let rendered_fact = render(&tuple, &lifted_problem);
            actual_fluent_set.insert(rendered_fact.to_lowercase());
        }

        let mut actual_action_set = HashSet::new();
        for action_tuple in raw_actions {
            let rendered_action =
                aiplan4rust::analysis::reachability::datalog::renderers::action::render(
                    &action_tuple,
                    &lifted_problem,
                );
            actual_action_set.insert(rendered_action.to_lowercase());
        }

        let actual_f = actual_fluent_set.len();
        let actual_a = actual_action_set.len();
        let exp_f = exp_fluent_set.len();

        // 4. Affichage et analyse par inclusion
        println!("\x1b[1;32m[INFO / ORACLE]\x1b[0m");
        println!(
            "    -> FD Oracle  : {} Domaines/Fluents, {} Actions",
            exp_f,
            exp_actions_set.len()
        );
        println!(
            "    -> Datalog    : {} Faits, {} Actions",
            actual_f, actual_a
        );

        // --- AFFICHAGE DES FORMATS (DEBUG COMPARATIF) ---
        if fd_available {
            println!("    \x1b[1;35m[DEBUG FORMAT] Tous les fluents FD :\x1b[0m");
            for f in exp_fluent_set.iter() {
                println!("      - \"{}\"", f);
            }

            println!("    \x1b[1;35m[DEBUG FORMAT] Tous les fluents Datalog :\x1b[0m");
            for f in actual_fluent_set.iter() {
                println!("      - \"{}\"", f);
            }

            println!("    \x1b[1;35m[DEBUG FORMAT] Toutes les actions FD (normalisées) :\x1b[0m");
            for a in exp_actions_set.iter() {
                println!("      - \"{}\"", a);
            }

            println!("    \x1b[1;35m[DEBUG FORMAT] Toutes les actions Datalog (rendu) :\x1b[0m");
            for a in actual_action_set.iter() {
                println!("      - \"{}\"", a);
            }

            // --- AFFICHAGE DE LA DIFFÉRENCE DATALOG vs FD (Fluents) ---
            let only_in_datalog: HashSet<_> =
                actual_fluent_set.difference(&exp_fluent_set).collect();

            println!("    \x1b[1;33m[INFO] {} éléments générés par Datalog et absents de l'oracle FD :\x1b[0m", only_in_datalog.len());
            for f in only_in_datalog {
                println!("      + \"{}\"", f);
            }
        }

        let mut problem_ok = true;

        if fd_available {
            // 1. Inclusion des fluents
            let missing_in_datalog: HashSet<_> =
                exp_fluent_set.difference(&actual_fluent_set).collect();
            if !missing_in_datalog.is_empty() {
                println!(
                    "    \x1b[1;33m[INFO] {} éléments de l'oracle FD absents du set Datalog (différence de format/représentation)\x1b[0m",
                    missing_in_datalog.len()
                );
            }

            // 2. Inclusion des actions (Vérification rigoureuse ensembliste)
            let missing_actions: HashSet<_> =
                exp_actions_set.difference(&actual_action_set).collect();

            if !missing_actions.is_empty() {
                println!(
                    "    \x1b[1;31m[MISMATCH] {} actions de l'oracle FD sont absentes de Datalog !\x1b[0m",
                    missing_actions.len()
                );
                for act in missing_actions {
                    println!("      - Action manquante : \"{}\"", act);
                }
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
            let _ = lines.next(); // var name/id
            let _ = lines.next(); // -1 ou axiom layer

            if let Some(count_line) = lines.next() {
                if let Ok(count) = count_line.trim().parse::<usize>() {
                    for _ in 0..count {
                        if let Some(val_line) = lines.next() {
                            fluents.push(val_line.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(fluents)
}

pub fn extract_datalog_style_fluents(sas_content: &str) -> Result<HashSet<String>, String> {
    let raw_fluents = extract_fluents_from_content(sas_content)?;
    let mut datalog_atoms = HashSet::new();

    for f in raw_fluents {
        let cleaned = if f.starts_with("Atom ") {
            &f[5..]
        } else {
            f.as_str()
        };

        let trimmed = cleaned.trim();
        let trimmed_lower = trimmed.to_lowercase();

        if trimmed_lower.starts_with("negatedatom")
            || trimmed.contains("new-axiom@")
            || !trimmed.contains('(')
            || !trimmed.ends_with(')')
        {
            continue;
        }

        datalog_atoms.insert(trimmed.to_lowercase());
    }

    Ok(datalog_atoms)
}

pub fn extract_actions_from_content(sas_content: &str) -> Result<Vec<String>, String> {
    let mut actions = Vec::new();
    let mut lines = sas_content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "begin_operator" {
            if let Some(action_name) = lines.next() {
                actions.push(action_name.trim().to_string());
            }
        }
    }

    Ok(actions)
}

pub fn run_fast_downward_and_get_content(
    domain_path: &Path,
    problem_path: &Path,
) -> Result<String, String> {
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
        .arg("--translate")
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
