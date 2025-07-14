use std::io::Read;
use std::path::Path;
use test_case::test_case;

mod common;
use crate::common::io::{collect_domain_files, delete_all_files_with_extension, write_ast_to_file, write_diagnostics_to_file, write_error_diagnostic_file};
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
    let mut success = true;

    // Delete all .diag files before running tests
    delete_all_files_with_extension(domain_dir, "diag");
    // Delete all .ast files before running tests
    delete_all_files_with_extension(domain_dir, "ast");

    // Collect all domain files
    let files = collect_domain_files(domain_dir);

    for file_path in files {
        // Read file content
        let content = read_file(&file_path);

        // Create parser instance
        let mut parser = Parser::new();

        // Parse file content
        let path_str = file_path
            .to_str()
            .expect("File path is not valid UTF-8");

        let parse_result = parser.parse(path_str, &content, language);

        match parse_result {
            Ok(mut parser_result) => {
                if let Some(raw_ast) = parser_result.take_ast() {
                    // Check raw AST well-formedness
                    if let Err(e) = check_well_formed(&raw_ast) {
                        eprintln!("Raw AST validation failed for {}:\n{}", file_path.display(), e);
                        success = false;
                        write_diagnostics_to_file(
                            parser_result.diagnostic_manager(),
                            &file_path,
                            "Raw AST validation error"
                        );
                        write_ast_to_file(&raw_ast, &file_path, "Normalizer tests: Raw AST validation error");
                        continue;
                    }

                    let diagnostic_manager = parser_result.take_diagnostic_manager();
                    let mut normalizer = Normalizer::new();

                    let normalize_result = normalizer.normalize_with_diagnostic_manager(raw_ast, diagnostic_manager);

                    match normalize_result {
                        Ok(mut normalizer_result) => {
                            if let Some(normalized_ast) = normalizer_result.take_ast() {
                                if let Err(e) = check_well_normalized(&normalized_ast) {
                                    eprintln!("Normalized AST validation failed for {}:\n{}", file_path.display(), e);
                                    eprintln!("{}", normalized_ast.to_string_with_interner());
                                    success = false;
                                    write_diagnostics_to_file(
                                        normalizer_result.diagnostic_manager(),
                                        &file_path,
                                        "Normalizer tests: Normalized AST validation error"
                                    );
                                    write_ast_to_file(&normalized_ast, &file_path, "Normalizer tests: Normalized AST validation error");
                                } else {
                                    write_diagnostics_to_file(
                                        normalizer_result.diagnostic_manager(),
                                        &file_path,
                                        "Normalizer tests: Normalization success"
                                    );
                                }
                            } else {
                                eprintln!("No normalized AST returned for file {}", file_path.display());
                                success = false;
                                write_diagnostics_to_file(
                                    normalizer_result.diagnostic_manager(),
                                    &file_path,
                                    "Normalizer tests: Normalization failed (no AST)"
                                );
                            }

                            let diag_mgr = normalizer_result.take_diagnostic_manager();
                            if diag_mgr.has_diagnostics_of_severity(Severity::Error) {
                                eprintln!("Diagnostics errors found for file {}", file_path.display());
                                success = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("Normalization error for file {}: {}", file_path.display(), e);
                            success = false;
                            // Write .diag manually for normalization error
                            write_error_diagnostic_file(
                                &file_path,
                                "Normalizer tests: Normalization error",
                                &e.to_string()
                            );
                        }
                    }
                } else {
                    eprintln!("Parsing failed (no AST) for file {}", file_path.display());
                    success = false;
                    write_diagnostics_to_file(
                        parser_result.diagnostic_manager(),
                        &file_path,
                        "Normalizer tests: Parsing failed (no AST)"
                    );
                }
            }
            Err(e) => {
                eprintln!("Parsing error for file {}: {}", file_path.display(), e);
                success = false;
                // Write .diag manually for parsing error
                write_error_diagnostic_file(
                    &file_path,
                    "Normalizer tests: Parsing error",
                    &e.to_string()
                );
            }
        }
    }

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
