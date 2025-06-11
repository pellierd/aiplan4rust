use std::{fs, io};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use test_case::test_case;

use aiplan4rust::{Language, Parser};

/// Collects all `.pddl` or `.hddl` files from the specified directory.
///
/// # Arguments
///
/// * `domain_dir` - A reference to a path representing the directory to search.
///
/// # Returns
///
/// A `Result` containing a vector of `PathBuf` if successful, or an `io::Error`.
pub fn collect_domain_files(domain_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(domain_dir)?;

    let mut files = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("pddl") || ext.eq_ignore_ascii_case("hddl") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Reads the content of a file into a string.
///
/// # Arguments
///
/// * `path` - A reference to the file path to be read.
///
/// # Returns
///
/// A `Result` containing the file's content as a `String`, or an `io::Error`.
pub fn read_file(path: &Path) -> io::Result<String> {
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist"));
    }

    let mut source = String::new();
    let mut file = fs::File::open(path)?;
    file.read_to_string(&mut source)?;
    Ok(source)
}

/// Parses all PDDL or HDDL files in a directory using the given language syntax.
///
/// # Arguments
///
/// * `domain_dir` - A reference to the directory containing the domain files.
/// * `language` - The language to use for parsing (`Language::PDDL` or `Language::HDDL`).
///
/// # Returns
///
/// A `bool` indicating whether all files were parsed successfully.
pub fn test_parse_all_files(domain_dir: &Path, language: &Language) -> bool {
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
            Ok(parser_result) => {
                if parser_result.syntax_tree().is_none() {
                    eprintln!("Parsing failed (no syntax tree) for file {}", file_path.display());
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

/// Test the syntax on a set of HDDL domains.
///
/// The function verifies that all files in the domain are successfully parsed.
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
