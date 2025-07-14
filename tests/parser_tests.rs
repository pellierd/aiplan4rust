use aiplan4rust::{check_well_formed, Language, Parser};
use std::io::Read;
use std::path::Path;
use test_case::test_case;

mod common;
use crate::common::io::collect_domain_files;
use crate::common::io::read_file;

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
/// * `language` - Language context to use for parsing.
///
/// # Returns
///
/// * `true` if parsing and validation succeed for all files.
/// * `false` if any file fails to parse or validate.
pub fn test_parse_all_files(domain_dir: &Path, language: &Language) -> bool {
    let mut success = true;
    // Collect all domain files in the directory
    let files = collect_domain_files(domain_dir);

    // Iterate over each file path
    for file_path in files {
        // Read the content of the current file
        let content = read_file(&file_path);

        // Create a new parser instance
        let mut parser = Parser::new();

        // Attempt to parse the file content
        let path_str = file_path
            .to_str()
            .expect("File path is not valid UTF-8");

        let parse_result = parser.parse(path_str, &content, language);

        // Match on the parsing result
        match parse_result {
            Ok(parser_result) => {
                // Check if AST is available from parsing
                if let Some(ast) = parser_result.ast() {
                    // Validate the well-formedness of the AST
                    match check_well_formed(ast) {
                        Ok(()) => {
                            println!("Validation successful: no errors.");
                        }
                        Err(e) => {
                            eprintln!("Validation failed:\n{}", e);
                            success = false;
                        }
                    }
                } else {
                    // No AST returned, parsing failed
                    eprintln!(
                        "Parsing failed (no syntax arena) for file {}",
                        file_path.display()
                    );
                    success = false;
                }
            }
            Err(e) => {
                // Parsing error occurred
                eprintln!("Parsing error for file {}: {}", file_path.display(), e);
                success = false;
            }
        }
    }

    // Return overall success status
    success
}

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
/// test_hddl_parser("tests/integration/hddl/ipc20/partial-order/barman-bdi");
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
pub fn test_hddl_parser(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_parse_all_files(path, &Language::HDDL),
        "Parsing test failed for directory {}",
        domain_path
    );
}
