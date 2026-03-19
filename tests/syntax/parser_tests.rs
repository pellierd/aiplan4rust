use std::path::Path;
use test_case::test_case;

use crate::common::io::*;
use crate::common::pipeline::*;

/// Tests the syntax correctness of all HDDL domain files in the specified directory.
///
/// This function iterates over all files in the given `domain_path` that belong to HDDL domains,
/// and attempts to parse each one.
///
/// The test will fail if any file fails to parse successfully, indicating a syntax error or parsing issue.
///
/// # Arguments
///
/// * `domain_path` - A string slice representing the path to the directory containing HDDL domain files.
///
/// # Panics
///
/// Panics if parsing fails for at least one file in the directory.
///
/// # Examples
///
/// ```
/// test_hddl_parser("tests/fixtures/hddl/ipc20/partial-order/barman-bdi");
/// ```
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
#[test_case("tests/fixtures/hddl/ipc20/total-order/blocksworld-hpddl"; "ipc20_total_order_blocksworld_hpdl")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/childsnack"; "ipc20_total_order_childsnack")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/depots"; "ipc20_total_order_depots")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/elevator-learned-ecai-16"; "ipc20_total_order_elevator_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/entertainment"; "ipc20_total_order_entertainment")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/factories-simple"; "ipc20_total_order_factories-simple")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/freecell-learned-ecai-16"; "ipc20_total_order_freecell-learned-ecai-16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/hiking"; "ipc20_total_order_hiking")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/logistics-learned-ecai-16"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-player"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-regular"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-fully-observable"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-partially-observable"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/multiarm-blocksworld"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/robot"; "ipc20_total_order_multiarm_robot")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/rover-gtohp"; "ipc20_total_order_rover_gtoph")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/satellite-gtohp"; "ipc20_total_order_satellite_gtoph")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/snake"; "ipc20_total_order_snake")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/towers"; "ipc20_total_order_towers")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/transport"; "ipc20_total_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/woodworking"; "ipc20_total_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/ultralight-cockpit"; "ipc23_partial_order_ultralight_cockpit")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/colouring"; "ipc23_partial_order_colouring")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/lamps"; "ipc23_total_order_lamps")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/sharpsat"; "ipc23_total_order_sharpsat")]
pub fn test_hddl_parser(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_parse_all_files(path),
        "Parsing test failed for directory {}",
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
pub fn test_pddl_parser(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_parse_all_files(path),
        "PDDL Parsing test failed for directory {}",
        domain_path
    );
}

/// Attempts to parse all domain files in the given directory for the specified language.
///
/// Iterates over each file collected from `domain_dir`, reads its content, parses it,
/// and validates the well-formedness of the resulting AST.
///
/// Returns `true` if all files are parsed and validated successfully, `false` otherwise.
///
/// # Arguments
///
/// * `domain_dir` - Path to the directory containing domain files.
///
/// # Returns
///
/// * `true` if parsing and validation succeed for all files.
/// * `false` if any file fails to parse or validate.
pub fn test_parse_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage des anciens fichiers de diagnostic et d'AST
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");

    // 2. Collecte de tous les fichiers disponibles et tri
    let mut all_files = collect_domain_files(domain_dir);
    all_files.sort();
    let total_available = all_files.len();

    // 3. Sélection des fichiers selon le mode (Swallow par défaut / Full si FULL_TESTS est mis)
    // Cette fonction 'filter_files_by_mode' est celle que nous avons isolée dans common
    let files_to_process = filter_files_by_mode(all_files);

    // 4. Boucle d'exécution du parsing
    for file_path in &files_to_process {
        match parse_and_check_ast(file_path) {
            Some(parser_result) => {
                let diag_mgr = parser_result.diagnostic_manager();
                let interner = parser_result.interner();

                if parser_result.is_success() {
                    // Succès : on écrit le diagnostic de réussite
                    write_diagnostics_to_file(
                        diag_mgr,
                        interner,
                        file_path,
                        "Parser Tests: parsing success",
                    );
                } else {
                    // Échec sémantique ou syntaxique : log et marquage de l'échec
                    eprintln!("\x1b[1;31mParsing Error:\x1b[0m {}", file_path.display());
                    write_diagnostics_to_file(
                        diag_mgr,
                        interner,
                        file_path,
                        "Parser Tests: parsing incomplete or errors found",
                    );
                    success = false;
                }
            }
            None => {
                // Erreur fatale (déjà rapportée par le pipeline)
                success = false;
            }
        }
    }

    // 5. Affichage du statut final (Swallow vs Full) via l'utilitaire commun
    print_test_status(files_to_process.len(), total_available, domain_dir);

    success
}
