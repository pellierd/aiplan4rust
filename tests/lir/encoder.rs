use std::path::Path;
use test_case::test_case;

use crate::common::io::{
    collect_domain_files, delete_all_files_with_extension, filter_problem_files,
    find_associated_domain, get_test_files_for_mode, print_test_status,
};
use crate::common::pipeline::{analyze_file, encode, link};

/// Integration test for LIR Builder on benchmark directories
#[test_case("tests/fixtures/hddl/ipc20/partial-order/barman-bdi"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/monroe-fully-observable"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/monroe-partially-observable"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/pcp"; "ipc20_partial_order_pcp")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/rover"; "ipc20_partial_order_rover")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/satellite"; "ipc20_partial_order_satellite")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/transport"; "ipc20_partial_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/um-translog"; "ipc20_partial_order_um_translog")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/woodworking"; "ipc20_partial_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/assembly-hierarchical"; "ipc20_total_order_assembly_hierarchical")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/barman-bdi"; "ipc20_total_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/blocksworld-gtohp"; "ipc20_total_order_blocksworld_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/blocksworld-hpddl"; "ipc20_total_order_blocksworld_hpddl")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/childsnack"; "ipc20_total_order_childsnack")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/depots"; "ipc20_total_order_depots")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/elevator-learned-ecai-16"; "ipc20_total_order_elevator_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/entertainment"; "ipc20_total_order_entertainment")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/factories-simple"; "ipc20_total_order_factories_simple")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/freecell-learned-ecai-16"; "ipc20_total_order_freecell_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/hiking"; "ipc20_total_order_hiking")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/logistics-learned-ecai-16"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-player"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-regular"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-fully-observable"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-partially-observable"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/multiarm-blocksworld"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/robot"; "ipc20_total_order_robot")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/rover-gtohp"; "ipc20_total_order_rover_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/satellite-gtohp"; "ipc20_total_order_satellite_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/snake"; "ipc20_total_order_snake")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/towers"; "ipc20_total_order_towers")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/transport"; "ipc20_total_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/woodworking"; "ipc20_total_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/ultralight-cockpit"; "ipc23_partial_order_ultralight_cockpit")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/colouring"; "ipc23_partial_order_colouring")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/lamps"; "ipc23_total_order_lamps")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/sharpsat"; "ipc23_total_order_sharpsat")]
pub fn test_hddl_encoder(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_lir_encode_all_files(path),
        "LIR Encoder integration test failed for directory {}",
        domain_path
    );
}

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
#[test_case("tests/fixtures/pddl/ipc02/rovers/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_rovers")]
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal/strips"; "ipc04_pddl_temporal_strips_airport")]
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows/strips"; "ipc04_pddl_temporal_timewindows_strips_airport")]
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows-compiled/strips"; "ipc04_pddl_temporal_timewindows_compiled_strips_airport")]
pub fn test_pddl_encoder(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_lir_encode_all_files(path),
        "LIR Encoder integration test failed for directory {}",
        domain_path
    );
}

/// Test the LIR Builder on all problems in a domain directory
pub fn test_lir_encode_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage global (ast, diag, linking et lir)
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");
    delete_all_files_with_extension(domain_dir, "linking.diag");
    delete_all_files_with_extension(domain_dir, "lir.diag");

    // 2. Collecte & Filtrage via common (Swallow/Full)
    let all_files = collect_domain_files(domain_dir);
    let all_problems = filter_problem_files(&all_files);
    let total_problems_available = all_problems.len();

    // Sélection selon le mode via ta fonction commune
    let problems_to_process = get_test_files_for_mode(all_problems);

    for problem_path in &problems_to_process {
        // Identification du domaine (gère .hddl et .pddl automatiquement)
        let domain_path = match find_associated_domain(problem_path) {
            Some(path) => path,
            None => {
                eprintln!(
                    "\x1b[1;31mError:\x1b[0m No domain found for {}",
                    problem_path.display()
                );
                success = false;
                continue;
            }
        };

        // --- PIPELINE JUSQU'AU LIR ---

        // Stage 1: Analyse Sémantique
        // On passe &mut success pour que l'échec soit marqué si l'analyse échoue
        let domain_ana = analyze_file(&domain_path, "domain", &mut success);
        let problem_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (domain_ana, problem_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                success = false;
                continue;
            }
        };

        // Stage 2: Linking
        let linking_result = match link(d_res, p_res, &domain_path, problem_path) {
            Some(res) => res,
            None => {
                eprintln!(
                    "\x1b[1;31mLinking failed\x1b[0m for {}",
                    problem_path.display()
                );
                success = false;
                continue;
            }
        };

        // Stage 3: LIR Encoding
        if encode(linking_result, &domain_path, problem_path).is_none() {
            eprintln!(
                "\x1b[1;31mLIR Encoding failed\x1b[0m for {}",
                problem_path.display()
            );
            success = false;
        }
    }

    // 4. Rapport de statut unifié (Cyan en Swallow, Vert en Full)
    print_test_status(
        problems_to_process.len(),
        total_problems_available,
        domain_dir,
    );

    success
}
