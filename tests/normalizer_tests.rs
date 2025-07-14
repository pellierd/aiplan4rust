use std::io::Read;
use std::path::Path;
use test_case::test_case;

mod common;
use crate::common::io::collect_domain_files;
use crate::common::io::read_file;

use aiplan4rust::aiplan4rust::diagnostic::Severity;
use aiplan4rust::aiplan4rust::validation::normalization::check_well_normalized;
use aiplan4rust::{check_well_formed, Language, Normalizer, Parser};

/// Integration test for parser + normalizer on all files in a directory.
///
/// Iterates over all domain files in the specified directory, performing:
/// 1. Parsing each file.
/// 2. Validating the well-formedness of the raw AST.
/// 3. Normalizing the AST.
/// 4. Validating the well-normalized AST.
/// 5. Checking for diagnostics errors.
///
/// Returns `true` if parsing and normalization succeed without critical errors for all files,
/// otherwise returns `false`.
///
/// # Arguments
///
/// * `domain_dir` - Path to the directory containing domain files to test.
/// * `language` - The language used for parsing.
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
pub fn test_parse_and_normalize_all_files(domain_dir: &Path, language: &Language) -> bool {
    // Initialize success flag to true; will be set to false if any error occurs
    let mut success = true;

    // Collect all domain files (.pddl or .hddl) from the specified directory
    let files = collect_domain_files(domain_dir);

    // Iterate over each collected file path
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

        // Handle the result of parsing
        match parse_result {
            // If parsing was successful
            Ok(mut parser_result) => {
                // Attempt to extract the raw AST from the parser result
                if let Some(raw_ast) = parser_result.take_ast() {
                    // Check that the raw AST is well-formed before normalization
                    if let Err(e) = check_well_formed(&raw_ast) {
                        eprintln!(
                            "Raw AST validation failed for {}:\n{}",
                            file_path.display(),
                            e
                        );
                        // Mark failure and continue to next file
                        success = false;
                        continue;
                    }

                    // Extract diagnostic manager from parser result
                    let diagnostic_manager = parser_result.take_diagnostic_manager();
                    // Create a new normalizer instance
                    let mut normalizer = Normalizer::new();

                    // Normalize the raw AST with diagnostics
                    let normalize_result =
                        normalizer.normalize_with_diagnostic_manager(raw_ast, diagnostic_manager);

                    // Handle normalization result
                    match normalize_result {
                        Ok(mut normalizer_result) => {
                            // Extract normalized AST if present
                            if let Some(normalized_ast) = normalizer_result.take_ast() {
                                // Check that the normalized AST is well-formed
                                if let Err(e) = check_well_normalized(&normalized_ast) {
                                    eprintln!(
                                        "Normalized AST validation failed for {}:\n{}",
                                        file_path.display(),
                                        e
                                    );
                                    eprintln!("{}", normalized_ast.to_string_with_interner());
                                    // Mark failure
                                    success = false;
                                }
                            } else {
                                // No normalized AST returned — mark failure
                                eprintln!(
                                    "No normalized AST returned for file {}",
                                    file_path.display()
                                );
                                success = false;
                            }
                            // Check diagnostic manager for any errors
                            let diag_mgr = normalizer_result.take_diagnostic_manager();
                            if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                                eprintln!(
                                    "Diagnostics errors found for file {}",
                                    file_path.display()
                                );
                                success = false;
                            }
                        }
                        // Normalization failed — log and mark failure
                        Err(e) => {
                            eprintln!(
                                "Normalization error for file {}: {}",
                                file_path.display(),
                                e
                            );
                            success = false;
                        }
                    }
                } else {
                    // Parsing succeeded but no AST was produced — mark failure
                    eprintln!("Parsing failed (no AST) for file {}", file_path.display());
                    success = false;
                }
            }
            // Parsing failed — log error and mark failure
            Err(e) => {
                eprintln!("Parsing error for file {}: {}", file_path.display(), e);
                success = false;
            }
        }
    }

    // Return overall success status (true if all files passed)
    success
}

/// Combined parser + normalizer integration test on an HDDL directory.
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
/// indicating a problem in the parser + normalizer pipeline.
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
/// Panics if parsing or normalization fails for any file.
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
pub fn test_hddl_normalizer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_parse_and_normalize_all_files(path, &Language::HDDL),
        "Parser + Normalizer integration test failed for directory {}",
        domain_path
    );
}
