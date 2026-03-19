use std::path::Path;
use test_case::test_case;
use aiplan4rust::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::common::io::*;
use crate::common::pipeline::*;

/// Tests ValueRegistry consistency (object counting per type) for a given directory.
pub fn test_registry_consistency(domain_dir: &Path) -> bool {
    let mut success = true;

    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!("\n\x1b[1;36m>>> Starting ValueRegistry Consistency Test in: {}\x1b[0m", domain_dir.display());

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        // --- Compilation Pipeline ---
        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => { success = false; continue; }
        };

        let linking = link(d_res, p_res, &domain_path, problem_path).expect("Link failed");
        let mut lir_result = encode(linking, &domain_path, problem_path).expect("Encoding failed");
        let pb = lir_result.take_lifted_problem().expect("No lifted problem");

        // --- ValueRegistry Building ---
        // Building the registry specifically for this problem instance
        let registry = ValueRegistry::build(pb.type_defs(), pb.object_defs()).unwrap();

        // --- Oracle: Expected object counts per Type (including inheritance) ---
        let expectations: Vec<(&str, usize)> = match oracle_key.as_str() {
            "gripper/prob01.pddl" => vec![
                ("room", 2),    // room1, room2
                ("ball", 4),    // ball1, ball2, ball3, ball4
                ("gripper", 2), // left, right
                ("object", 8),  // Total flattened
            ],
            "logistics/prob01.pddl" | "logistics/pb01.pddl" | "logistics/p01.pddl" => vec![
                ("obj", 6),       // The 6 packages
                ("city", 6),      // The 6 cities
                ("truck", 6),     // The 6 trucks
                ("airplane", 2),  // The 2 planes
                ("airport", 6),   // The 6 city-X-2 locations
                ("location", 12), // The 6 city-X-1 + the 6 city-X-2
                ("object", 32),   // Grand total
            ],
            "movie/prob01.pddl" | "movie/pb01.pddl" => vec![
                ("chips", 5),
                ("dip", 5),
                ("pop", 5),
                ("cheese", 5),
                ("crackers", 5),
                ("object", 25),
            ],
            "assembly/pb01.pddl" | "assembly/prob01.pddl" => vec![
                ("assembly", 5),
                ("resource", 2),
                ("object", 7),
            ],
            "depot/prob01.pddl" | "depot/pb01.pddl" => vec![
                ("place", 3),
                ("truck", 2),
                ("hoist", 3),
                ("surface", 5),
                ("object", 13),
            ],
            _ => vec![],
        };

        if expectations.is_empty() {
            println!("  \x1b[0;90mSkipping {} (No registry oracle defined)\x1b[0m", oracle_key);
            continue;
        }

        print!("  Checking Registry {}... ", oracle_key);

        let mut current_problem_ok = true;

        for (type_name, expected_count) in expectations {
            // 1. Get the SymbolId from the string name via interner
            let type_symbol = pb.interner().lookup_symbol(type_name)
                .expect(&format!("Type name '{}' not found in interner", type_name));

            // 2. Get the TypeId from the SymbolId via type_symbols map
            let t_id = pb.type_symbols().try_get_id(&type_symbol)
                .expect(&format!("Type symbol for '{}' not found in type definitions", type_name));

            // 3. Query the registry for the flattened domain
            match registry.get_primitive_type_domain(t_id) {
                Ok(domain) => {
                    let actual_count = domain.len();
                    if actual_count != expected_count {
                        if current_problem_ok {
                            println!("\x1b[1;31m[FAILED]\x1b[0m");
                            current_problem_ok = false;
                        }
                        println!("    \x1b[0;31m- Type '{}' (ID:{:?}) expected {} objects, but registry found {}\x1b[0m",
                                 type_name, t_id, expected_count, actual_count);
                        success = false;
                    }
                },
                Err(e) => {
                    if current_problem_ok {
                        println!("\x1b[1;31m[FAILED]\x1b[0m");
                        current_problem_ok = false;
                    }
                    println!("    \x1b[0;31m- Type '{}' : Registry error: {:?}\x1b[0m", type_name, e);
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

// --- Test Entry Points ---

#[test_case("tests/fixtures/pddl/ipc98/assembly/adl/"; "registry_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips/"; "registry_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips/"; "registry_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips/"; "registry_movie")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/barman-bdi"; "registry_barman")]
pub fn test_pddl_value_registry(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(test_registry_consistency(path), "ValueRegistry consistency failed for domain: {}", domain_path);
}
