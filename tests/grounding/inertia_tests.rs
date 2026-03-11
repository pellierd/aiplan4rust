use std::path::Path;
use test_case::test_case;

use crate::common::io::*;
use crate::common::pipeline::*;

use aiplan4rust::type_flattening;
use aiplan4rust::aiplan4rust::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
use aiplan4rust::aiplan4rust::lang::AtomSkeletonId;

/// Teste la cohérence de la table d'inertie pour un répertoire de domaine donné.
pub fn test_inertia_consistency(domain_dir: &Path) -> bool {
    let mut success = true;

    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!("\n\x1b[1;36m>>> Starting Inertia Consistency Test in: {}\x1b[0m", domain_dir.display());

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        // --- Pipeline de compilation ---
        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => { success = false; continue; }
        };

        let linking = link(d_res, p_res, &domain_path, problem_path).expect("Link failed");
        let mut lir_result = encode(linking, &domain_path, problem_path).expect("Encoding failed");
        let mut pb = lir_result.take_lifted_problem().expect("No lifted problem");

        // --- Transformations ---
        type_flattening::flatten(&mut pb).unwrap();

        // --- Analyse d'Inertie ---
        let table = analyze_inertia(&pb).unwrap();

        // --- Oracle : Définition des attentes par domaine ---
        // Format: (Nom du prédicat, Fonction de vérification, Label attendu)
        let expectations: Vec<(&str, fn(Inertia) -> bool, &str)> = match oracle_key.as_str() {
            "assembly/pb01.pddl" | "assembly/prob01.pddl" => vec![
                ("requires", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("available", |i| i.is_fluent(), "FLUENT"),
                ("part-of", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("complete", |i| i.is_fluent(), "FLUENT"),
                ("incorporated", |i| i.is_fluent(), "FLUENT"),
            ],
            "gripper/prob01.pddl" => vec![
                ("at-robby", |i| i.is_fluent(), "FLUENT"),
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("carry", |i| i.is_fluent(), "FLUENT"),
            ],
            "logistics/prob01.pddl" | "logistics/pb01.pddl" | "logistics/p01.pddl" => vec![
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("in", |i| i.is_fluent(), "FLUENT"),
                ("in-city", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("obj", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("truck", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("location", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("airplane", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("city", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("airport", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            "movie/prob01.pddl" | "movie/pb01.pddl" => vec![
                ("movie-rewound", |i| i.is_fluent(), "FLUENT"),
                ("counter-at-zero", |i| i.is_fluent(), "FLUENT"),
                ("have-chips", |i| i.is_fluent(), "FLUENT"),
                ("have-dip", |i| i.is_fluent(), "FLUENT"),
                ("have-pop", |i| i.is_fluent(), "FLUENT"),
                ("have-cheese", |i| i.is_fluent(), "FLUENT"),
                ("have-crackers", |i| i.is_fluent(), "FLUENT"),
                ("chips", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("dip", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("pop", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("cheese", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("crackers", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("counter-at-two-hours", |i| i.is_negative(), "NEGATIVE_INERTIA"),
                ("counter-at-other-than-two-hours", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            "mystery/prob01.pddl" | "mystery/pb01.pddl" => vec![
                ("craves", |i| i.is_fluent(), "FLUENT"),
                ("harmony", |i| i.is_fluent(), "FLUENT"),
                ("locale", |i| i.is_fluent(), "FLUENT"),
                ("fears", |i| i.is_negative(), "NEGATIVE_INERTIA"),
                ("food", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("pleasure", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("pain", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("province", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("planet", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("eats", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("attacks", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("orbits", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            "mprime/prob01.pddl" | "mprime/pb01.pddl" => vec![
                ("craves", |i| i.is_fluent(), "FLUENT"),
                ("harmony", |i| i.is_fluent(), "FLUENT"),
                ("locale", |i| i.is_fluent(), "FLUENT"),
                ("fears", |i| i.is_negative(), "NEGATIVE_INERTIA"),
                ("food", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("pleasure", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("pain", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("province", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("planet", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("eats", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("attacks", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("orbits", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            "barman/prob01.pddl" | "barman/pb01.pddl" => vec![
                ("contains", |i| i.is_fluent(), "FLUENT"),
                ("clean", |i| i.is_fluent(), "FLUENT"),
                ("empty", |i| i.is_fluent(), "FLUENT"),
                ("holding", |i| i.is_fluent(), "FLUENT"),
                ("handEmpty", |i| i.is_fluent(), "FLUENT"),
                ("ontable", |i| i.is_fluent(), "FLUENT"),
                ("used", |i| i.is_fluent(), "FLUENT"),
                ("shaked", |i| i.is_fluent(), "FLUENT"),
                ("unshaked", |i| i.is_fluent(), "FLUENT"),
                ("shakerLevel", |i| i.is_fluent(), "FLUENT"),
                ("cocktailPart1", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("cocktailPart2", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("dispenses", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("next", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("shakerEmptyLevel", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            "depot/prob01.pddl" | "depot/pb01.pddl" => vec![
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("on", |i| i.is_fluent(), "FLUENT"),
                ("in", |i| i.is_fluent(), "FLUENT"),
                ("lifting", |i| i.is_fluent(), "FLUENT"),
                ("available", |i| i.is_fluent(), "FLUENT"),
                ("clear", |i| i.is_fluent(), "FLUENT"),
                ("current_load", |i| i.is_fluent(), "FLUENT"),
                ("fuel-cost", |i| i.is_fluent(), "FLUENT"),
                ("load_limit", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
                ("weight", |i| i.is_positive_negative(), "POSITIVE_NEGATIVE_INERTIA"),
            ],
            _ => vec![],
        };

        if expectations.is_empty() {
            println!("  \x1b[0;90mSkipping {} (No oracle defined)\x1b[0m", oracle_key);
            continue;
        }

        print!("  Checking {}... ", oracle_key);

        let mut current_problem_ok = true;

        for (pred_name, check_fn, expected_label) in expectations {
            // 1. On cherche la position du prédicat en résolvant l'ID via le registre
            let predicate_pos = pb.predicate_defs().iter().position(|p| {
                // 1. Récupère l'identifiant interne du symbole (PredicateSymbolId)
                let pred_symbol_id = p.symbol();

                // 2. Récupère l'Ident (le token) associé à cet ID dans le registre
                if let Some(ident) = pb.predicate_symbols().get_ident(pred_symbol_id) {
                    // 3. Résout le nom texte via l'interner et compare avec le nom attendu
                    pb.interner().resolve_symbol(*ident) == Some(pred_name)
                } else {
                    false
                }
            });

            if let Some(pos) = predicate_pos {
                let id = AtomSkeletonId::from(pos);
                let inertia = table.get_predicate(id).expect("Predicate missing in table");

                if !check_fn(inertia) {
                    if current_problem_ok {
                        println!("\x1b[1;31m[FAILED]\x1b[0m");
                        current_problem_ok = false;
                    }
                    println!("    \x1b[0;31m- Predicate '{}' (ID:{:?}) expected {}, but table says {}\x1b[0m",
                             pred_name, id, expected_label, inertia);
                    success = false;
                }
            } else {
                println!("\n    \x1b[0;33m- Warning: Predicate '{}' not found in problem definition\x1b[0m", pred_name);
            }
        }

        if current_problem_ok {
            println!("\x1b[1;32m[PASS]\x1b[0m");
        }
    }

    success
}

// --- Points d'entrée des tests ---

#[test_case("tests/fixtures/pddl/ipc98/assembly/adl/"; "ipc98_pddl_adl_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips/"; "ipc98_pddl_strips_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips/"; "ipc98_pddl_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips/"; "ipc98_pddl_strips_movie")]
#[test_case("tests/fixtures/pddl/ipc98/mystery/strips/"; "ipc98_pddl_strips_mystery")]
#[test_case("tests/fixtures/pddl/ipc98/mystery-prime/strips/"; "ipc98_pddl_strips_mystery_prime")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/barman-bdi"; "ipc20_total_order_barman_bdi")]
pub fn test_pddl_inertia_table(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(test_inertia_consistency(path), "Inertia table consistency failed for domain: {}", domain_path);
}
