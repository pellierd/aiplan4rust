use std::fs::File;
use std::io::Write;
use std::path::Path;
use test_case::test_case;

use aiplan4rust::{Renderer, Frontend};

mod common;
use crate::common::io::collect_domain_files;
use crate::common::io::filter_problem_files;
use crate::common::io::get_file_stem_as_string;

/// Tests parsing and validation of a set of domain and problem HDDL files in a directory.
///
/// This function:
/// - Collects all `.hddl` files in the given directory.
/// - Filters files to find problem definitions (files starting with "pb").
/// - For each problem file:
///     - Locates the corresponding domain file (`<stem>-domain.hddl` or `domain.hddl`).
///     - Parses the domain and problem together.
///     - Writes diagnostics to a `.diag` file.
///     - Reports any errors encountered.
///
/// Returns `true` if all problem/domain pairs were successfully parsed and validated.
fn test_domain(domain_dir: &Path) -> bool {
    // Collect all files in the directory
    let files = collect_domain_files(domain_dir);

    // Filter problem files (those starting with "pb")
    let mut problem_files = filter_problem_files(&files);

    // Sort for consistent processing order
    problem_files.sort_by_key(|p| p.file_name().map(|f| f.to_os_string()));

    let mut success = true;

    // Iterate over each problem file
    for problem_path in problem_files {
        // Extract the stem (base filename) to identify the domain file
        let problem_stem = get_file_stem_as_string(&problem_path);

        // Build possible domain file paths
        let domain_path1 = domain_dir.join("domain.hddl");
        let domain_path2 = domain_dir.join(format!("{}-domain.hddl", problem_stem));

        // Determine which domain file exists
        let domain_path = if domain_path2.exists() {
            domain_path2
        } else if domain_path1.exists() {
            domain_path1
        } else {
            eprintln!(
                "No domain file found for problem: {}",
                problem_path.display()
            );
            success = false;
            continue;
        };

        // Log which files will be parsed
        println!(
            "\x1b[1;36mParsing:\x1b[0m \n - {} \n - {}",
            domain_path.display(),
            problem_path.display()
        );

        // Initialize the parser frontend
        let frontend = Frontend::new();
        let result = frontend.parse(
            domain_path.to_str().unwrap(),
            problem_path.to_str().unwrap(),
        );

        // Prepare a diagnostics output file
        let diag_path = domain_dir.join(format!("{}.diag", problem_stem));
        let mut diag_file = File::create(&diag_path)
            .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));

        match result {
            Ok(builder_result) => {
                // Write diagnostics to file
                let mut buffer = Vec::new();
                Renderer::write_to(builder_result.diagnostic_manager(), builder_result.interner(), &mut buffer, false)
                    .expect("Failed to write diagnostics");
                diag_file.write_all(&buffer).expect("Failed to write to diag file");

                // Check whether the parsing produced a lifted problem
                if builder_result.lifted_problem().is_none() {
                    eprintln!(
                        "\x1b[1;36m===> Failure:\x1b[0m {}",
                        diag_path.display()
                    );
                    success = false;
                }
            }
            Err(e) => {
                // Handle and log parsing error
                let err_msg = format!(
                    "Parsing error for domain: {} and problem: {}.\nError: {}\n",
                    domain_path.display(),
                    problem_path.display(),
                    e
                );
                eprintln!("{}", err_msg);
                diag_file
                    .write_all(err_msg.as_bytes())
                    .expect("Failed to write error to diag file");
                success = false;
            }
        }
    }

    success
}

/// Generates one test per domain directory found under `tests/integration/hddl`.
///
/// Each `#[test_case]` annotation defines a separate test that is discovered and run by `cargo test`.
/// This allows systematic testing of all known HDDL benchmark domains.
///
/// The test will:
/// - Run `test_domain()` on the directory.
/// - Fail if any problem/domain pair in the directory fails parsing or validation.
///
/// # Arguments
/// * `domain_path` - The path to the domain directory to test.
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
fn test_each_domain(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_domain(path),
        "Domain test failed for {}",
        domain_path
    );
}
