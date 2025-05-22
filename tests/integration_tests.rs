use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use test_case::test_case;

use aiplan4rust::{DiagnosticRenderer, Frontend, Language};

fn test_domain(domain_dir: &Path, language: &Language) -> bool {
    let files = fs::read_dir(domain_dir)
        .expect("Cannot read domain subdir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map(|s| s == "hddl").unwrap_or(false))
        .collect::<Vec<_>>();

    let mut problem_files: Vec<_> = files.iter()
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with("pb") && !f.contains("-domain"))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    problem_files.sort_by_key(|p| p.file_name().map(|f| f.to_os_string()));

    let mut success = true;

    for problem_path in problem_files {
        let problem_stem = problem_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let domain_path1 = domain_dir.join("domain.hddl");
        let domain_path2 = domain_dir.join(format!("{}-domain.hddl", problem_stem));
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

        println!(
            "\x1b[1;36mParsing:\x1b[0m \n - {} \n - {}",
            domain_path.display(),
            problem_path.display()
        );

        let frontend = Frontend::new();
        let result = frontend.parse(
            domain_path.to_str().unwrap(),
            problem_path.to_str().unwrap(),
            language,
        );

        let diag_path = domain_dir.join(format!("{}.diag", problem_stem));
        let mut diag_file = File::create(&diag_path)
            .unwrap_or_else(|_| panic!("Failed to create diag file: {}", diag_path.display()));

        match result {
            Ok(linker_result) => {
                let mut buffer = Vec::new();
                DiagnosticRenderer::write_to(linker_result.diagnostic_manager(), &mut buffer, false)
                    .expect("Failed to write diagnostics");
                diag_file.write_all(&buffer).expect("Failed to write to diag file");

                if linker_result.planning_task().is_none() {
                    eprintln!(
                        "\x1b[1;36m===> Failure:\x1b[0m {}",
                        diag_path.display()
                    );
                    success = false;
                }
            }
            Err(e) => {
                let err_msg = format!(
                    "Parsing error for domain: {} and problem: {}.\nError: {}\n",
                    domain_path.display(),
                    problem_path.display(),
                    e
                );
                eprintln!("{}", err_msg);
                diag_file.write_all(err_msg.as_bytes()).expect("Failed to write error to diag file");
                success = false;
            }
        }
    }

    success
}

// Génère un test par domaine trouvé dans le dossier
// Chaque `#[test_case]` crée un test distinct dans `cargo test`
#[test_case("tests/integration/hddl/ipc20/partial-order/barman-bdi"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/integration/hddl/ipc20/partial-order/colouring"; "ipc20_partial_order_colouring")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-fully-observable"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/monroe-partially-observable"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/integration/hddl/ipc20/partial-order/pcp"; "ipc20_partial_order_pcp")]
#[test_case("tests/integration/hddl/ipc20/partial-order/rover"; "ipc20_partial_order_rover")]
#[test_case("tests/integration/hddl/ipc20/partial-order/satellite"; "ipc20_partial_order_satellite")]
#[test_case("tests/integration/hddl/ipc20/partial-order/transport"; "ipc20_partial_order_transport")]
#[test_case("tests/integration/hddl/ipc20/partial-order/ultralight-cockpit"; "ipc20_partial_order_ultralight-cockpit")]
fn test_each_domain(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_domain(path, &Language::HDDL),
        "Domain test failed for {}",
        domain_path
    );
}
