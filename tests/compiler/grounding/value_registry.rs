use std::path::Path;
use test_case::test_case;

use crate::common::compiler::*;
use crate::common::io::*;
use aiplan4rust::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;

/// Asserts the consistency of the built ValueRegistry (object counts per type) against an oracle.
pub fn test_registry_consistency(domain_dir: &Path) -> bool {
    let mut success = true;

    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running ValueRegistry Consistency Suite: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let mut comps = problem_path.components();

        let _tests = comps.next();
        let _fixtures = comps.next();
        let _kind = comps.next(); // pddl / hddl
        let _ipc = comps.next(); // ipcXX

        let domain = comps
            .next()
            .expect("Missing domain component")
            .as_os_str()
            .to_string_lossy()
            .to_string();

        let problem = problem_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .replace("-domain", "");

        let oracle_key = format!("{}/{}", domain, problem);
        let domain_path = find_associated_domain(problem_path).expect("Domain file not found");

        // --- Compilation Pipeline ---
        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                success = false;
                continue;
            }
        };

        let linking = link(d_res, p_res, &domain_path, problem_path).expect("Linking failed");
        let mut lir_result =
            encode(linking, &domain_path, problem_path).expect("LIR encoding failed");
        let pb = lir_result
            .take_lifted_problem()
            .expect("Lifted problem extraction failed");

        // --- ValueRegistry Construction ---
        let registry =
            ValueRegistry::build(pb.type_defs().as_slice(), pb.object_defs().as_slice()).unwrap();

        // --- Oracle Expectations (Flattened object counts per type with inheritance) ---
        let expectations: Vec<(&str, usize)> = match oracle_key.as_str() {
            "gripper/pb01" => vec![("room", 2), ("ball", 4), ("gripper", 2), ("object", 8)],
            "logistics/pb01" => vec![
                ("obj", 6),
                ("city", 6),
                ("truck", 6),
                ("airplane", 2),
                ("airport", 6),
                ("location", 12),
                ("object", 32),
            ],
            "movie/pb01" => vec![
                ("chips", 5),
                ("dip", 5),
                ("pop", 5),
                ("cheese", 5),
                ("crackers", 5),
                ("object", 25),
            ],
            "assembly/pb01" => vec![("assembly", 19), ("resource", 2), ("object", 21)],
            "depot/pb01" => vec![
                ("place", 3),
                ("truck", 2),
                ("hoist", 3),
                ("surface", 5),
                ("object", 13),
            ],
            _ => vec![],
        };

        if expectations.is_empty() {
            println!(
                "  \x1b[0;90mSkipping {} (No registry oracle defined)\x1b[0m",
                oracle_key
            );
            continue;
        }

        print!("  Verifying Registry {}... ", oracle_key);
        let mut current_problem_ok = true;

        for (type_name, expected_count) in expectations {
            // Locate the Type identifier safely through the symbol maps
            let type_symbol = match pb.interner().lookup_symbol(type_name) {
                Some(sym) => sym,
                None => {
                    log::warn!(
                        "Type token '{}' missing from interner cache map for context: {}",
                        type_name,
                        oracle_key
                    );
                    continue;
                }
            };

            let t_id = match pb.type_symbols().try_get_id(&type_symbol) {
                Ok(id) => id,
                Err(e) => {
                    log::warn!("Type structural symbol mapping unresolved for token identifier '{}' in domain {}: {:?}", type_name, oracle_key, e);
                    continue;
                }
            };

            // Query domain mapping inside the registry
            match registry.get_primitive_type_domain(t_id) {
                Ok(domain) => {
                    let actual_count = domain.len();
                    if actual_count != expected_count {
                        if current_problem_ok {
                            println!("\x1b[1;31m[FAILED]\x1b[0m");
                            current_problem_ok = false;
                        }
                        println!("    \x1b[0;31m- Type '{}' (ID:{:?}) expected {} objects, registry counted {}\x1b[0m",
                                 type_name, t_id, expected_count, actual_count);
                        success = false;
                    }
                }
                Err(e) => {
                    if current_problem_ok {
                        println!("\x1b[1;31m[FAILED]\x1b[0m");
                        current_problem_ok = false;
                    }
                    println!(
                        "    \x1b[0;31m- Type '{}' evaluated runtime evaluation panic: {:?}\x1b[0m",
                        type_name, e
                    );
                    success = false;
                }
            }
        }

        if current_problem_ok {
            println!("\x1b[1;32m[PASS]\x1b[0m");
        }
    }

    success
}

// --- Test Suite Entrypoints ---

#[test_case("tests/fixtures/pddl/ipc98/assembly/adl/"; "registry_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips/"; "registry_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips/"; "registry_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips/"; "registry_movie")]
pub fn test_pddl_value_registry(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(
        test_registry_consistency(path),
        "ValueRegistry domain size verification crashed for scope: {}",
        domain_path
    );
}
