use std::path::Path;
use test_case::test_case;

mod common;
use crate::common::io::{collect_domain_files, delete_all_files_with_extension, filter_files_by_mode, print_test_status};
use crate::common::pipeline::{normalize_and_check_ast, parse_and_check_ast};

/// Integration test for parser + simplification on all files in a directory.
///
/// Iterates over all domain files in the specified directory, performing:
/// 1. Parsing each file.
/// 2. Validating the well-formedness of the raw AST.
/// 3. Normalizing the AST.
/// 4. Validating the well-normalized AST.
/// 5. Checking for diagnostics errors.
///
/// Returns `true` if parsing and expr succeed without critical errors for all files,
/// otherwise returns `false`.
///
/// # Arguments
///
/// * `domain_dir` - Path to the directory containing domain files to test.
///
/// # Errors
///
/// Instead of panicking, this function logs errors and continues processing all files,
/// aggregating success/failure.
///
/// # Examples
///
/// ```
/// let success = test_parse_and_normalize_all_files(Path::new("tests/integration/hddl/ipc20/partial-order/barman-bdi"), &Language::HDDL);
/// assert!(success);
/// ```
pub fn test_normalizer_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage des anciens fichiers
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");

    // 2. Collecte et tri
    let mut all_files = collect_domain_files(domain_dir);
    all_files.sort();
    let total_available = all_files.len();

    // 3. Sélection intelligente (Swallow par défaut / Full si FULL_TESTS=1)
    let files_to_process = filter_files_by_mode(all_files);

    for file_path in &files_to_process {
        // Étape 1 : Parsing
        let parser_result = match parse_and_check_ast(file_path) {
            Some(result) => result,
            None => {
                eprintln!("\x1b[1;31mParsing failed\x1b[0m for file {}", file_path.display());
                success = false;
                continue;
            }
        };

        // Étape 2 : Normalisation
        // On utilise la même logique : si ça renvoie None, c'est un échec
        if normalize_and_check_ast(parser_result, file_path).is_none() {
            eprintln!("\x1b[1;31mNormalization failed\x1b[0m for file {}", file_path.display());
            success = false;
            continue;
        }
    }

    // 4. Rapport de statut uniforme
    print_test_status(files_to_process.len(), total_available, domain_dir);

    success
}


/// Combined parser + simplification integration test on an HDDL directory.
///
/// This test iterates over all files in the given `domain_path` directory
/// corresponding to HDDL domains and performs for each file:
///
/// 1. Parsing the file.
/// 2. Checking the validity of the raw AST.
/// 3. Normalizing the AST.
/// 4. Checking the validity of the normalized AST.
///
/// The test fails (panics) if any error occurs during any of these steps,
/// indicating a problem in the parser + simplification pipeline.
///
/// # Arguments
///
/// * `domain_path` - Path to a directory containing HDDL domain files.
///
/// # Examples
///
/// ```
/// test_hddl_normalizer("tests/integration/hddl/ipc20/partial-order/barman-bdi");
/// ```
///
/// # Notes
///
/// The test is automatically invoked for multiple predefined test directories
/// via the `#[test_case]` attributes.
///
/// # Panics
///
/// Panics if parsing or expr fails for any file.
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
#[test_case("tests/integration/hddl/ipc20/total-order/factories-simple"; "ipc20_total_order_factories-simple")]
#[test_case("tests/integration/hddl/ipc20/total-order/freecell-learned-ecai-16"; "ipc20_total_order_freecell-learned-ecai-16")]
#[test_case("tests/integration/hddl/ipc20/total-order/hiking"; "ipc20_total_order_hiking")]
#[test_case("tests/integration/hddl/ipc20/total-order/logistics-learned-ecai-16"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/integration/hddl/ipc20/total-order/minecraft-player"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/integration/hddl/ipc20/total-order/minecraft-regular"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/integration/hddl/ipc20/total-order/monroe-fully-observable"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/integration/hddl/ipc20/total-order/monroe-partially-observable"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/integration/hddl/ipc20/total-order/multiarm-blocksworld"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/integration/hddl/ipc20/total-order/robot"; "ipc20_total_order_multiarm_robot")]
#[test_case("tests/integration/hddl/ipc20/total-order/rover-gtohp"; "ipc20_total_order_rover_gtoph")]
#[test_case("tests/integration/hddl/ipc20/total-order/satellite-gtohp"; "ipc20_total_order_satellite_gtoph")]
#[test_case("tests/integration/hddl/ipc20/total-order/snake"; "ipc20_total_order_snake")]
#[test_case("tests/integration/hddl/ipc20/total-order/towers"; "ipc20_total_order_towers")]
#[test_case("tests/integration/hddl/ipc20/total-order/transport"; "ipc20_total_order_transport")]
#[test_case("tests/integration/hddl/ipc20/total-order/woodworking"; "ipc20_total_order_woodworking")]
#[test_case("tests/integration/hddl/ipc23/partial-order/ultralight-cockpit"; "ipc23_partial_order_ultralight_cockpit")]
#[test_case("tests/integration/hddl/ipc23/partial-order/colouring"; "ipc23_partial_order_colouring")]
#[test_case("tests/integration/hddl/ipc23/total-order/lamps"; "ipc23_total_order_lamps")]
pub fn test_hddl_normalizer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_normalizer_all_files(path),
        "Parser + Normalizer integration test failed for directory {}",
        domain_path
    );
}

#[test_case("tests/integration/pddl/ipc98/assembly"; "ipc98_pddl_adl_assembly")]
#[test_case("tests/integration/pddl/ipc98/gripper/adl"; "ipc98_pddl_adl_gripper")]
#[test_case("tests/integration/pddl/ipc98/gripper/strips"; "ipc98_pddl_strips_gripper")]
#[test_case("tests/integration/pddl/ipc98/logistics/adl"; "ipc98_pddl_adl_logistics")]
#[test_case("tests/integration/pddl/ipc98/logistics/strips"; "ipc98_pddl_strips_logistics")]
#[test_case("tests/integration/pddl/ipc98/movie/adl"; "ipc98_pddl_adl_movie")]
#[test_case("tests/integration/pddl/ipc98/movie/strips"; "ipc98_pddl_strips_movie")]
#[test_case("tests/integration/pddl/ipc98/mystery-prime/strips"; "ipc98_pddl_strips_mystery_prime")]
#[test_case("tests/integration/pddl/ipc98/mystery/strips"; "ipc98_pddl_strips_mystery")]
pub fn test_pddl_normalizer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_normalizer_all_files(path),
        "PDDL Parsing test failed for directory {}",
        domain_path
    );
}
