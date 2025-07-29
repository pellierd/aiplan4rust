use test_case::test_case;

use std::io::Write;
use std::path::{Path};

use aiplan4rust::{Language};
mod common;
use crate::common::io::{collect_domain_files, delete_all_files_with_extension, filter_problem_files, get_file_stem_as_string, write_linking_diag_to_file};
use crate::common::pipeline::{link, analyze_file};

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
/// - `language`: The language context used during parsing and analysis.
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
pub fn test_linker_all_files(domain_dir: &Path, language: &Language) -> bool {
    // Remove old diagnostic and AST files
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");
    common::io::delete_all_files_with_extension(domain_dir, "linking.diag");

    // Collect all domain files
    let all_files = collect_domain_files(domain_dir);
    let mut problem_files = filter_problem_files(&all_files);
    problem_files.sort_by_key(|p| p.file_name().map(|f| f.to_os_string()));

    let mut success = true;

    for problem_path in problem_files {
        let problem_stem = get_file_stem_as_string(&problem_path);

        // Select domain file: either <problem>-domain.hddl or domain.hddl
        let domain_path1 = domain_dir.join("domain.hddl");
        let domain_path2 = domain_dir.join(format!("{}-domain.hddl", problem_stem));
        let domain_path = if domain_path2.exists() {
            domain_path2
        } else if domain_path1.exists() {
            domain_path1
        } else {
            eprintln!("No domain file found for problem: {}", problem_path.display());
            success = false;
            continue;
        };

        // Analyze domain file
        let domain = match analyze_file(&domain_path, language, "domain", &mut success) {
            Some(res) => res,
            None => continue,
        };

        // Analyze problem file
        let problem = match analyze_file(&problem_path, language, "problem", &mut success) {
            Some(res) => res,
            None => continue,
        };

        // Perform linking (this function writes its own diagnostics)
        let linking_result = link(domain, problem, &domain_path, &problem_path);

        // Check for linking errors
        if linking_result.is_none() {
            eprintln!(
                "Linking failed for problem {} and domain {}",
                problem_path.display(),
                domain_path.display()
            );
            success = false;
        }
    }

    success
}

/// Integration test for linking HDDL domain and problem files in a given directory.
///
/// This test is parameterized with many known benchmark directories (e.g., IPC domains).
/// It ensures that parsing, normalization, semantic analysis, and linking succeed without errors
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
/// This function will panic if any step (parsing, normalization, analysis, or linking)
/// fails for any file in the directory. The panic message includes the directory
/// that caused the failure.
///
/// # Example
///
/// ```rust
/// test_hddl_linker("tests/integration/hddl/ipc20/total-order/barman-bdi");
/// ```
#[test_case("tests/integration/hddl/ipc20/partial-order/barman-bdi"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/integration/hddl/ipc20/partial-order/colouring"; "ipc20_partial_order_colouring")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-fully-observable"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-partially-observable"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/pcp"; "ipc20_partial_order_pcp")]
#[test_case("tests/integration/hddl/ipc20/partial-order/rover"; "ipc20_partial_order_rover")]
#[test_case("tests/integration/hddl/ipc20/partial-order/satellite"; "ipc20_partial_order_satellite")]
#[test_case("tests/integration/hddl/ipc20/partial-order/transport"; "ipc20_partial_order_transport")]
#[test_case("tests/integration/hddl/ipc20/partial-order/ultralight-cockpit"; "ipc20_partial_order_ultralight_cockpit")]
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
pub fn test_hddl_linker(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_linker_all_files(path, &Language::HDDL),
        "Linking test failed for directory {}",
        domain_path
    );
}
