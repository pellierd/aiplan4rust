use test_case::test_case;

use std::path::{Path};

use crate::common::io::*;
use crate::common::pipeline::*;

/// Tests the linker on all problem files found in the given domain directory.
///
/// This function performs the following steps:
/// - Deletes all existing diagnostic (`.diag`), AST (`.ast`), and linking diagnostic (`.linking.diag`) files in the domain directory.
/// - Collects all domain-related files and filters out the problem files.
/// - For each problem file:
///   - Attempts to find an associated domain file, either `domain.hddl` or `<problem>-domain.hddl`.
///   - Analyzes both the domain and the problem files.
///   - Runs semantic linking using the `link` function (which writes its own diagnostics).
///   - Checks for linking errors and logs failures.
///
/// # Parameters
///
/// - `domain_dir`: Path to the directory containing domain and problem files.
///
/// # Returns
///
/// - `true` if all files linked successfully without errors.
/// - `false` if any linking or analysis failure occurred.
///
/// # Side Effects
///
/// - Writes and deletes diagnostic files inside the provided domain directory.
/// - Prints error messages to standard error for failures.
///
/// # Example
///
/// ```ignore
/// let domain_dir = Path::new("path/to/domain");
/// let language = Language::HDDL;
/// let all_ok = test_linker_all_files(domain_dir, &language);
/// if !all_ok {
///     eprintln!("Some linking tests failed.");
/// }
/// ```
pub fn test_linker_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "linking.diag");

    // 2. Collecte & Filtrage via common
    let all_files = collect_domain_files(domain_dir);
    let all_problems = filter_problem_files(&all_files);
    let problems_to_process = get_test_files_for_mode(all_problems);

    for problem_path in &problems_to_process {
        let domain_path = match find_associated_domain(problem_path) {
            Some(path) => path,
            None => {
                eprintln!("\x1b[1;31mError:\x1b[0m No domain found for {}", problem_path.display());
                success = false;
                continue;
            }
        };

        // 3. Pipeline
        let domain_ana = match analyze_file(&domain_path, "domain", &mut success) {
            Some(res) => res,
            None => continue,
        };

        let problem_ana = match analyze_file(problem_path, "problem", &mut success) {
            Some(res) => res,
            None => continue,
        };

        if link(domain_ana, problem_ana, &domain_path, problem_path).is_none() {
            success = false;
        }
    }

    print_test_status(problems_to_process.len(), filter_problem_files(&all_files).len(), domain_dir);
    success
}

/// Integration test for linking HDDL domain and problem files in a given directory.
///
/// This test is parameterized with many known benchmark directories (e.g., IPC domains).
/// It ensures that parsing, logic, semantic analysis, and linking succeed without errors
/// for every domain/problem pair in the specified directory.
///
/// Each test case corresponds to a directory path containing HDDL files to link.
///
/// # Arguments
///
/// * `domain_path` - A string slice representing the path to the directory containing HDDL files.
///
/// # Panics
///
/// This function will panic if any step (parsing, logic, analysis, or linking)
/// fails for any file in the directory. The panic message includes the directory
/// that caused the failure.
///
/// # Example
///
/// ```rust
/// test_hddl_linker("tests/fixtures/hddl/ipc20/total-order/barman-bdi");
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
pub fn test_hddl_linker(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_linker_all_files(path),
        "Linking test failed for directory {}",
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
#[test_case("tests/fixtures/pddl/ipc02/depot/numeric_automatic/"; "ipc02_pddl_numeric_automatic_depot")]
pub fn test_pddl_linker(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_linker_all_files(path),
        "Linking test failed for directory {}",
        domain_path
    );
}
