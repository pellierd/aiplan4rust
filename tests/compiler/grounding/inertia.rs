use std::collections::HashSet;
use std::path::Path;
use test_case::test_case;

use crate::common::compiler::*;
use crate::common::io::*;

use crate::common::fd::{
    extract_fluents_from_content, extract_predicate_names, run_fast_downward_and_get_content,
};
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::support::lang::AtomSkeletonId;

/// Asserts the consistency of the generated inertia table against Fast Downward dynamically.
pub fn test_inertia_consistency(domain_dir: &Path) -> bool {
    let mut success = true;

    // Nettoyage préalable des fichiers de diagnostic éventuels
    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running Dynamic FD Inertia Consistency Suite: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_path = find_associated_domain(problem_path).expect("Domain file not found");

        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Verifying {}... ", oracle_key);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // 1. Lancement de Fast Downward et récupération dynamique des fluents de l'oracle
        let fd_fluent_preds = match run_fast_downward_and_get_content(&domain_path, problem_path) {
            Ok(content) => match extract_fluents_from_content(&content) {
                Ok(fluents) => extract_predicate_names(&fluents),
                Err(_) => HashSet::new(),
            },
            Err(e) => {
                println!("\x1b[1;33m[WARNING] Impossible de lancer FD: {}\x1b[0m", e);
                HashSet::new()
            }
        };

        // Nettoyage du fichier output.sas généré par FD
        let _ = std::fs::remove_file("output.sas");

        if fd_fluent_preds.is_empty() {
            println!("  \x1b[0;90mSkipping (No FD fluents extracted or FD failed)\x1b[0m");
            continue;
        }

        // 2. --- Pipeline de Compilation ---
        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                println!("\x1b[1;31m[FAILED (Parsing)]\x1b[0m");
                success = false;
                continue;
            }
        };

        let linking = match link(d_res, p_res, &domain_path, problem_path) {
            Some(l) => l,
            None => {
                println!("\x1b[1;31m[FAILED (Linking)]\x1b[0m");
                success = false;
                continue;
            }
        };

        let mut lir_result = match encode(linking, &domain_path, problem_path) {
            Some(res) => res,
            None => {
                println!("\x1b[1;31m[FAILED (Encoding)]\x1b[0m");
                success = false;
                continue;
            }
        };

        // --- Fetch Lifted Problem Representation ---
        let pb = lir_result
            .take_lifted_problem()
            .expect("Lifted problem extraction failed");

        // --- Generate Inertia Table ---
        let table = match analyze_inertia(&pb) {
            Ok(t) => t,
            Err(e) => {
                println!("\x1b[1;31m[FAILED (Inertia Table)]\x1b[0m: {}", e);
                success = false;
                continue;
            }
        };

        let mut current_problem_ok = true;

        // 3. --- Vérification par inclusion (Lifted vs Oracle FD) ---
        for (pos, pred_def) in pb.predicate_defs().iter().enumerate() {
            let pred_symbol_id = pred_def.symbol();
            let pred_name = if let Some(ident) = pb.predicate_symbols().get_ident(pred_symbol_id) {
                pb.interner()
                    .resolve_symbol(*ident)
                    .unwrap_or("unknown")
                    .to_lowercase()
            } else {
                continue;
            };

            let id = AtomSkeletonId::from(pos);
            let inertia = table
                .get_predicate(id)
                .expect("Predicate registry mismatch");

            let should_be_fluent = fd_fluent_preds.contains(&pred_name);
            let is_pos_neg = inertia.is_positive_negative();
            let is_currently_fluent = inertia.is_fluent();
            let is_pos = inertia.is_positive();
            let is_neg = inertia.is_negative();

            // Affichage informatif et granulaire pour chaque prédicat testé
            println!(
                "    - Prédicat '{}' -> Type détecté : {} (Fluent: {}, PosNeg: {}, Pos: {}, Neg: {}) | FD Variable: {}",
                pred_name, inertia, is_currently_fluent, is_pos_neg, is_pos, is_neg, should_be_fluent
            );

            // --- RÈGLE D'INCLUSION (SOUNDNESS) ---

            // Cas 1 : Sous-approximation critique (ERREUR)
            // FD considère le prédicat comme changeant (dans les fluents SAS+),
            // mais mon analyse lifted le classe comme totalement statique pur (POSITIVE_NEGATIVE_INERTIA).
            if should_be_fluent && is_pos_neg {
                if current_problem_ok {
                    println!("\x1b[1;31m[FAILED]\x1b[0m");
                    current_problem_ok = false;
                }
                println!(
                    "    \x1b[0;31m- Sous-approximation critique : '{}' bouge selon FD, mais ton analyse le classe statique pur (POSITIVE_NEGATIVE_INERTIA)\x1b[0m",
                    pred_name
                );
                success = false;
            }
            // Cas 2 : Sur-approximation (ACCEPTÉE / INFO)
            // FD considère le prédicat comme statique, mais mon analyse le trouve non-statique (Fluent ou partiel).
            // C'est sécurisé (sound), donc on se contente d'un message informatif sans faire échouer le test.
            else if !should_be_fluent && !is_pos_neg {
                println!(
                    "    \x1b[0;33m[INFO] Sur-approximation (sûre) : '{}' est statique pour FD, mais classé comme {} par ton analyse\x1b[0m",
                    pred_name, inertia
                );
            }
        }

        if current_problem_ok {
            println!("\x1b[1;32m[PASS]\x1b[0m");
        }
    }
    success
}

// --- Test Suite Entrypoints ---

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
// IPC 2002
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/umt2"; "ipc02_pddl_umt2")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/automatic/typed"; "ipc02_pddl_typed_complex_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/automatic/untyped"; "ipc02_pddl_untyped_complex_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/handcoded/typed"; "ipc02_pddl_typed_complex_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/handcoded/untyped"; "ipc02_pddl_untyped_complex_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/hard-numeric/automatic/typed"; "ipc02_pddl_typed_hard_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/hard-numeric/automatic/untyped"; "ipc02_pddl_untyped_hard_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/automatic/typed"; "ipc02_pddl_typed_complex_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/automatic/untyped"; "ipc02_pddl_untyped_complex_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/handcoded/typed"; "ipc02_pddl_typed_complex_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/handcoded/untyped"; "ipc02_pddl_untyped_complex_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/hard-numeric/automatic/typed"; "ipc02_pddl_typed_hard_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/hard-numeric/automatic/untyped"; "ipc02_pddl_untyped_hard_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_freecell")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_freecell")]
#[test_case("tests/fixtures/pddl/ipc02/settlers/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_settlers")]
#[test_case("tests/fixtures/pddl/ipc02/settlers/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_settlers")]
// IPC 2004
#[test_case("tests/fixtures/pddl/ipc04/airport/nontemporal/adl"; "ipc04_pddl_nontemporal_adl_airport")]
#[test_case("tests/fixtures/pddl/ipc04/airport/nontemporal/strips"; "ipc04_pddl_nontemporal_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal/adl"; "ipc04_pddl_temporal_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal/strips"; "ipc04_pddl_temporal_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows/adl"; "ipc04_pddl_temporal_timewindows_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows/strips"; "ipc04_pddl_temporal_timewindows_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows-compiled/adl"; "ipc04_pddl_temporal_timewindows_compiled_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows-compiled/strips"; "ipc04_pddl_temporal_timewindows_compiled_strips_airport")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-nontemporal/strips"; "ipc04_pddl_notankage_nontemporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal/strips-temporal"; "ipc04_pddl_notankage_temporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal-deadlines/strips-temporal-timedliterals"; "ipc04_pddl_notankage_temporal_deadlines_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal-deadlines-co/strips-temporal"; "ipc04_pddl_notankage_temporal_deadlines_co_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/tankage-temporal/strips-temporal"; "ipc04_pddl_tankage_temporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/tankage-nontemporal/strips"; "ipc04_pddl_tankage_nontemporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph/adl"; "ipc04_pddl_optical_telegraph_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph/strips"; "ipc04_pddl_optical_telegraph_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-derivedpredic/adl-derivedpredicates"; "ipc04_pddl_optical_telegraph_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-derivedpredic/strips-derivedpredicates"; "ipc04_pddl_optical_telegraph_derivedpredicates_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-fluents/adl-fluents"; "ipc04_pddl_optical_telegraph_fluents_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-fluents-deriv/adl-fluents-derivedpredicates"; "ipc04_pddl_optical_telegraph_fluents_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers/adl"; "ipc04_pddl_philosophers_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers/strips"; "ipc04_pddl_philosophers_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-derivedpredicates/adl-derivedpredicates"; "ipc04_pddl_philosophers_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-derivedpredicates/strips-derivedpredicates"; "ipc04_pddl_philosophers_derivedpredicates_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-fluents/adl-fluents"; "ipc04_pddl_philosophers_fluents_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-fluents-derivedpre/adl-fluents-derivedpredicates"; "ipc04_pddl_philosophers_fluents_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/psr/large/adl-derivedpredicates"; "ipc04_pddl_large_derivedpredicates_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/adl-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/simple-adl-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_simple_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/strips-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_strips_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle-compiled/adl"; "ipc04_pddl_middle_compiled_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/small/strips"; "ipc04_pddl_small_strips_psr")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex/strips-fluents-temporal"; "ipc04_pddl_complex_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_complex_timewindows_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex-timewindows-compiled/strips-fluents-temporal"; "ipc04_pddl_complex_timewindows_compiled_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/numeric/strips-fluents"; "ipc04_pddl_numeric_strips_fluents_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/strips/strips"; "ipc04_pddl_strips_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time/strips-temporal"; "ipc04_pddl_time_strips_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time-windows/strips-temporal-timedliterals"; "ipc04_pddl_time_windows_strips_temporal_timedliterals_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time-windows-compiled/strips-temporal"; "ipc04_pddl_time_windows_compiled_strips_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/settlers/strips-fluents"; "ipc04_pddl_strips_fluents_settlers")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal/strips-fluents-temporal"; "ipc04_pddl_flaw_temporal_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_flaw_temporal_timewindows_strips_fluents_timedliterals_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal-timewindows-compi/strips-fluents-temporal"; "ipc04_pddl_flaw_temporal_timewindows_compiled_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal/strips-fluents-temporal"; "ipc04_pddl_temporal_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_temporal_timewindows_strips_fluents_timedliterals_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal-timewindows-compiled/strips-fluents-temporal"; "ipc04_pddl_temporal_timewindows_compiled_strips_fluents_umts")]
// IPC 2006
#[test_case("tests/fixtures/pddl/ipc06/openstacks/metric-time/adl"; "ipc06_pddl_openstacks_metric_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/metric-time/strips"; "ipc06_pddl_openstacks_metric_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/adl"; "ipc06_pddl_openstacks_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/strips"; "ipc06_pddl_openstacks_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/qualitative-preferences"; "ipc06_pddl_openstacks_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/simple-preferences"; "ipc06_pddl_openstacks_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/adl"; "ipc06_pddl_openstacks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/strips"; "ipc06_pddl_openstacks_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/complex-preferences"; "ipc06_pddl_pathways_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/metric-time"; "ipc06_pddl_pathways_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/adl"; "ipc06_pddl_pathways_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/strips"; "ipc06_pddl_pathways_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/simple-preferences"; "ipc06_pddl_pathways_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/complex-preferences"; "ipc06_pddl_pipesworld_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/metric-time"; "ipc06_pddl_pipesworld_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/metric-time-constraints"; "ipc06_pddl_pipesworld_metric_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/adl"; "ipc06_pddl_pipesworld_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/strips"; "ipc06_pddl_pipesworld_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/adl"; "ipc06_pddl_rovers_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/strips"; "ipc06_pddl_rovers_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/metric-simple-preferences"; "ipc06_pddl_rovers_metric_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/metric-time"; "ipc06_pddl_rovers_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/qualitative-preferences"; "ipc06_pddl_rovers_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/complex-preferences"; "ipc06_pddl_storage_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/propositional"; "ipc06_pddl_storage_propositional")]
#[test_case("tests/fixtures/pddl/ipc06/storage/qualitative-preferences"; "ipc06_pddl_storage_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/simple-preferences/grounded"; "ipc06_pddl_storage_simple_preferences_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/storage/simple-preferences/ungrounded"; "ipc06_pddl_storage_simple_preferences_ungrounded")]
#[test_case("tests/fixtures/pddl/ipc06/storage/time"; "ipc06_pddl_storage_time")]
#[test_case("tests/fixtures/pddl/ipc06/storage/time-constraints"; "ipc06_pddl_storage_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/complex-preferences"; "ipc06_pddl_TPP_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric"; "ipc06_pddl_TPP_metric")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric-time"; "ipc06_pddl_TPP_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric-time-constraints"; "ipc06_pddl_TPP_metric_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/adl"; "ipc06_pddl_TPP_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/strips"; "ipc06_pddl_TPP_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/qualitative-preferences"; "ipc06_pddl_TPP_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/simple-preferences/grounded"; "ipc06_pddl_TPP_simple_preferences_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/simple-preferences/ungrounded"; "ipc06_pddl_TPP_simple_preferences_ungrounded")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/complex-preferences"; "ipc06_pddl_trucks_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/adl"; "ipc06_pddl_trucks_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/strips"; "ipc06_pddl_trucks_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/qualitative-preferences"; "ipc06_pddl_trucks_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple"; "ipc06_pddl_trucks_simple")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple-grounded"; "ipc06_pddl_trucks_simple_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple-grounded-preferences"; "ipc06_pddl_trucks_simple_grounded_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/adl"; "ipc06_pddl_trucks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/strips"; "ipc06_pddl_trucks_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time-constraints/constraints"; "ipc06_pddl_trucks_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time-constraints/timed-initial-literals"; "ipc06_pddl_trucks_time_constraints_timed_initial_literals")]
// IPC 2008
// ===================== NETBEN-OPT =====================
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/crewplanning-numeric"; "ipc08_pddl_netben_opt_crewplanning_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/elevators-numeric"; "ipc08_pddl_netben_opt_elevators_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/elevators-strips"; "ipc08_pddl_netben_opt_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-adl"; "ipc08_pddl_netben_opt_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-numericadl"; "ipc08_pddl_netben_opt_openstacks_numericadl")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-stripsneg"; "ipc08_pddl_netben_opt_openstacks_stripsneg")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/pegsol-strips"; "ipc08_pddl_netben_opt_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/transport-numeric"; "ipc08_pddl_netben_opt_transport_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/woodworking-numeric"; "ipc08_pddl_netben_opt_woodworking_numeric")]
// ===================== SEQ-OPT =====================
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/elevators-strips"; "ipc08_pddl_seq_opt_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/openstacks-adl"; "ipc08_pddl_seq_opt_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/openstacks-strips"; "ipc08_pddl_seq_opt_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/parcprinter-strips"; "ipc08_pddl_seq_opt_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/pegsol-strips"; "ipc08_pddl_seq_opt_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/scanalyzer-strips"; "ipc08_pddl_seq_opt_scanalyzer_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/sokoban-strips"; "ipc08_pddl_seq_opt_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/transport-strips"; "ipc08_pddl_seq_opt_transport_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/woodworking-strips"; "ipc08_pddl_seq_opt_woodworking_strips")]
// ===================== SEQ-SAT =====================
//#[test_case("tests/fixtures/pddl/ipc08/seq-sat/cybersec-strips"; "ipc08_pddl_seq_sat_cybersec_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/elevators-strips"; "ipc08_pddl_seq_sat_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/openstacks-adl"; "ipc08_pddl_seq_sat_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/openstacks-strips"; "ipc08_pddl_seq_sat_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/parcprinter-strips"; "ipc08_pddl_seq_sat_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/pegsol-strips"; "ipc08_pddl_seq_sat_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/scanalyzer-strips"; "ipc08_pddl_seq_sat_scanalyzer_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/sokoban-strips"; "ipc08_pddl_seq_sat_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/transport-strips"; "ipc08_pddl_seq_sat_transport_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/woodworking-strips"; "ipc08_pddl_seq_sat_woodworking_strips")]
// ===================== TEMPO-SAT =====================
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/crewplanning-strips"; "ipc08_pddl_tempo_sat_crewplanning_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/elevators-numeric"; "ipc08_pddl_tempo_sat_elevators_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/elevators-strips"; "ipc08_pddl_tempo_sat_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/modeltrain-numeric"; "ipc08_pddl_tempo_sat_modeltrain_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-adl"; "ipc08_pddl_tempo_sat_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-numeric"; "ipc08_pddl_tempo_sat_openstacks_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-numericadl"; "ipc08_pddl_tempo_sat_openstacks_numericadl")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-strips"; "ipc08_pddl_tempo_sat_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/parcprinter-strips"; "ipc08_pddl_tempo_sat_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/pegsol-strips"; "ipc08_pddl_tempo_sat_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/sokoban-strips"; "ipc08_pddl_tempo_sat_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/transport-numeric"; "ipc08_pddl_tempo_sat_transport_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/woodworking-numeric"; "ipc08_pddl_tempo_sat_woodworking_numeric")]
pub fn test_pddl_inertia_table(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(
        test_inertia_consistency(path),
        "Inertia consistency evaluation failed for context: {}",
        domain_path
    );
}
