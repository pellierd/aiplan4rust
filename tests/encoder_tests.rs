use std::path::Path;
use test_case::test_case;

use aiplan4rust::LirEncoder;
mod common;
use crate::common::io::{collect_domain_files, delete_all_files_with_extension, filter_problem_files, get_file_stem_as_string};
use crate::common::pipeline::{analyze_file, encode, link};

/// Test the LIR Builder on all problems in a domain directory
pub fn test_lir_builder_all_files(domain_dir: &Path) -> bool {
    // 1. Nettoyage des anciens fichiers de diagnostic
    delete_all_files_with_extension(domain_dir, "linking.diag");
    delete_all_files_with_extension(domain_dir, "lir.diag");
    delete_all_files_with_extension(domain_dir, "ast");
    delete_all_files_with_extension(domain_dir, "diag");

    // 2. Collecte et tri des fichiers problèmes
    let all_files = collect_domain_files(domain_dir);
    let mut problem_files = filter_problem_files(&all_files);
    problem_files.sort_by_key(|p| p.file_name().map(|f| f.to_os_string()));

    let mut errors = Vec::new();

    for problem_path in problem_files {
        let problem_stem = get_file_stem_as_string(&problem_path);
        let mut success_flag = true; // Pour satisfaire la signature de analyze_file

        // 3. Identification du domaine correspondant
        let domain_path1 = domain_dir.join("domain.hddl");
        let domain_path2 = domain_dir.join(format!("{}-domain.hddl", problem_stem));
        let domain_path = if domain_path2.exists() { domain_path2 } else { domain_path1 };

        if !domain_path.exists() {
            errors.push(format!("No domain file found for {}", problem_path.display()));
            continue;
        }

        // --- DÉBUT DE LA PIPELINE ---

        // Stage 1: Analyse Sémantique (Domain & Problem)
        let domain_ana = analyze_file(&domain_path, "domain", &mut success_flag);
        let problem_ana = analyze_file(&problem_path, "problem", &mut success_flag);

        let (d_res, p_res) = match (domain_ana, problem_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                errors.push(format!("Analysis failed for domain/problem pair: {}", problem_stem));
                continue;
            }
        };

        // Stage 2: Linking
        let linking_result = match link(d_res, p_res, &domain_path, &problem_path) {
            Some(res) => res,
            None => {
                errors.push(format!("Linking failed for problem: {}", problem_path.display()));
                continue;
            }
        };

        // Stage 3: LIR Encoding
        match encode(linking_result, &domain_path, &problem_path) {
            Some(_) => {
                // Succès : le fichier .lir.diag a été écrit par encode()
            }
            None => {
                errors.push(format!("LIR Encoding produced no result for {}", problem_path.display()));
            }
        }
    }

    // 4. Rapport final
    if errors.is_empty() {
        true
    } else {
        eprintln!("\n==== LIR BUILDER ERRORS ====\n{}\n============================\n", errors.join("\n\n"));
        false
    }
}

/// Integration test for LIR Builder on benchmark directories
#[test_case("tests/integration/hddl/ipc20/partial-order/barman-bdi"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-fully-observable"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-partially-observable"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/pcp"; "ipc20_partial_order_pcp")]
#[test_case("tests/integration/hddl/ipc20/partial-order/rover"; "ipc20_partial_order_rover")]
#[test_case("tests/integration/hddl/ipc20/partial-order/satellite"; "ipc20_partial_order_satellite")]
#[test_case("tests/integration/hddl/ipc20/partial-order/transport"; "ipc20_partial_order_transport")]
#[test_case("tests/integration/hddl/ipc20/partial-order/um-translog"; "ipc20_partial_order_um_translog")]
#[test_case("tests/integration/hddl/ipc20/partial-order/woodworking"; "ipc20_partial_order_woodworking")]
#[test_case("tests/integration/hddl/ipc20/total-order/assembly-hierarchical"; "ipc20_total_order_assembly_hierarchical")]
#[test_case("tests/integration/hddl/ipc20/total-order/barman-bdi"; "ipc20_total_order_barman_bdi")]
#[test_case("tests/integration/hddl/ipc20/total-order/blocksworld-gtohp"; "ipc20_total_order_blocksworld_gtohp")]
#[test_case("tests/integration/hddl/ipc20/total-order/blocksworld-hpddl"; "ipc20_total_order_blocksworld_hpddl")]
#[test_case("tests/integration/hddl/ipc20/total-order/childsnack"; "ipc20_total_order_childsnack")]
#[test_case("tests/integration/hddl/ipc20/total-order/depots"; "ipc20_total_order_depots")]
#[test_case("tests/integration/hddl/ipc20/total-order/elevator-learned-ecai-16"; "ipc20_total_order_elevator_learned_ecai_16")]
#[test_case("tests/integration/hddl/ipc20/total-order/entertainment"; "ipc20_total_order_entertainment")]
#[test_case("tests/integration/hddl/ipc20/total-order/factories-simple"; "ipc20_total_order_factories_simple")]
#[test_case("tests/integration/hddl/ipc20/total-order/freecell-learned-ecai-16"; "ipc20_total_order_freecell_learned_ecai_16")]
#[test_case("tests/integration/hddl/ipc20/total-order/hiking"; "ipc20_total_order_hiking")]
#[test_case("tests/integration/hddl/ipc20/total-order/logistics-learned-ecai-16"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/integration/hddl/ipc20/total-order/minecraft-player"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/integration/hddl/ipc20/total-order/minecraft-regular"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/integration/hddl/ipc20/total-order/monroe-fully-observable"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/integration/hddl/ipc20/total-order/monroe-partially-observable"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/integration/hddl/ipc20/total-order/multiarm-blocksworld"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/integration/hddl/ipc20/total-order/robot"; "ipc20_total_order_robot")]
#[test_case("tests/integration/hddl/ipc20/total-order/rover-gtohp"; "ipc20_total_order_rover_gtohp")]
#[test_case("tests/integration/hddl/ipc20/total-order/satellite-gtohp"; "ipc20_total_order_satellite_gtohp")]
#[test_case("tests/integration/hddl/ipc20/total-order/snake"; "ipc20_total_order_snake")]
#[test_case("tests/integration/hddl/ipc20/total-order/towers"; "ipc20_total_order_towers")]
#[test_case("tests/integration/hddl/ipc20/total-order/transport"; "ipc20_total_order_transport")]
#[test_case("tests/integration/hddl/ipc20/total-order/woodworking"; "ipc20_total_order_woodworking")]
#[test_case("tests/integration/hddl/ipc23/partial-order/ultralight-cockpit"; "ipc23_partial_order_ultralight_cockpit")]
#[test_case("tests/integration/hddl/ipc23/partial-order/colouring"; "ipc23_partial_order_colouring")]
#[test_case("tests/integration/hddl/ipc23/total-order/lamps"; "ipc23_total_order_lamps")]
pub fn test_hddl_encoder(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_lir_builder_all_files(path),
        "LIR Builder integration test failed for directory {}",
        domain_path
    );
}
