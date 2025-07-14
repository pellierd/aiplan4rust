use std::io::Read;
use std::path::Path;
use test_case::test_case;

mod common;
use crate::common::io::read_file;
use crate::common::io::collect_domain_files;

use aiplan4rust::{check_well_formed, Language, Parser, Normalizer};
use aiplan4rust::aiplan4rust::diagnostic::Severity;
use aiplan4rust::aiplan4rust::validation::normalization::check_well_normalized;

/// Test d’intégration pour parser + normalizer sur tous les fichiers d’un répertoire.
///
/// Renvoie true si parsing et normalisation réussissent sans erreur critique.
pub fn test_parse_and_normalize_all_files(domain_dir: &Path, language: &Language) -> bool {
    let files = match collect_domain_files(domain_dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to collect domain files: {}", e);
            return false;
        }
    };

    let mut success = true;

    for file_path in files {
        let content = match read_file(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read file {}: {}", file_path.display(), e);
                success = false;
                continue;
            }
        };

        let mut parser = Parser::new();

        let parse_result = parser.parse(
            file_path.to_str().unwrap_or_default(),
            &content,
            language,
        );

        match parse_result {
            Ok(mut parser_result) => {
                if let Some(mut raw_ast) = parser_result.take_ast() {
                    // Check well-formedness of raw AST before normalization
                    if let Err(e) = check_well_formed(&raw_ast) {
                        eprintln!("Raw AST validation failed for {}:\n{}", file_path.display(), e);
                        success = false;
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
                                }
                            } else {
                                eprintln!("No normalized AST returned for file {}", file_path.display());
                                success = false;
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
                        }
                    }
                } else {
                    eprintln!("Parsing failed (no AST) for file {}", file_path.display());
                    success = false;
                }
            }
            Err(e) => {
                eprintln!("Parsing error for file {}: {}", file_path.display(), e);
                success = false;
            }
        }
    }

    success
}

/// Test d’intégration combiné parser + normalizer sur un répertoire HDDL.
///
/// Le test échoue si une erreur survient à l’une des étapes.
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
