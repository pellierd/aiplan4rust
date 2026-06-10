use std::path::Path;
use test_case::test_case;

use crate::common::compiler::*;
use crate::common::io::*;

// --- Grounding Analysis Imports ---
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::inertia::Inertia;
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::support::lang::AtomSkeletonId;

/// Asserts the consistency of the generated inertia table against an oracle for a given domain directory.
pub fn test_inertia_consistency(domain_dir: &Path) -> bool {
    let mut success = true;

    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;36m>>> Running Inertia Consistency Suite: {}\x1b[0m",
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

        // --- Fetch Lifted Problem Representation ---
        let pb = lir_result
            .take_lifted_problem()
            .expect("Lifted problem extraction failed");

        // --- Generate Inertia Table ---
        let table = analyze_inertia(&pb).expect("Inertia analysis computation failed");

        // --- Oracle Expectations Framework ---
        let expectations: Vec<(&str, fn(Inertia) -> bool, &str)> = match oracle_key.as_str() {
            "assembly/pb01" => vec![
                (
                    "requires",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                ("available", |i| i.is_fluent(), "FLUENT"),
                (
                    "part-of",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                ("complete", |i| i.is_fluent(), "FLUENT"),
                ("incorporated", |i| i.is_fluent(), "FLUENT"),
            ],
            "gripper/pb01" => vec![
                ("at-robby", |i| i.is_fluent(), "FLUENT"),
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("carry", |i| i.is_fluent(), "FLUENT"),
            ],
            "logistics/pb01" => vec![
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("in", |i| i.is_fluent(), "FLUENT"),
                (
                    "in-city",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "obj",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "truck",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "location",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "airplane",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "city",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "airport",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
            ],
            "movie/pb01" => vec![
                ("movie-rewound", |i| i.is_fluent(), "FLUENT"),
                ("counter-at-zero", |i| i.is_fluent(), "FLUENT"),
                ("have-chips", |i| i.is_fluent(), "FLUENT"),
                ("have-dip", |i| i.is_fluent(), "FLUENT"),
                ("have-pop", |i| i.is_fluent(), "FLUENT"),
                ("have-cheese", |i| i.is_fluent(), "FLUENT"),
                ("have-crackers", |i| i.is_fluent(), "FLUENT"),
                (
                    "chips",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "dip",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "pop",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "cheese",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "crackers",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "counter-at-two-hours",
                    |i| i.is_negative(),
                    "NEGATIVE_INERTIA",
                ),
                (
                    "counter-at-other-than-two-hours",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
            ],
            "mystery/pb01" | "mystery-prime/pb01" | "mprime/pb01" => vec![
                ("craves", |i| i.is_fluent(), "FLUENT"),
                ("harmony", |i| i.is_fluent(), "FLUENT"),
                ("locale", |i| i.is_fluent(), "FLUENT"),
                ("fears", |i| i.is_fluent(), "FLUENT"),
                (
                    "food",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "pleasure",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "pain",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "province",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "planet",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "eats",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "attacks",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "orbits",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
            ],
            "barman-bdi/pb01" => vec![
                ("contains", |i| i.is_fluent(), "FLUENT"),
                ("clean", |i| i.is_fluent(), "FLUENT"),
                ("empty", |i| i.is_fluent(), "FLUENT"),
                ("holding", |i| i.is_fluent(), "FLUENT"),
                ("handEmpty", |i| i.is_fluent(), "FLUENT"),
                ("ontable", |i| i.is_fluent(), "FLUENT"),
                ("used", |i| i.is_fluent(), "FLUENT"),
                ("shaked", |i| i.is_fluent(), "FLUENT"),
                ("unshaked", |i| i.is_fluent(), "FLUENT"),
                ("shakerLevel", |i| i.is_fluent(), "FLUENT"),
                (
                    "cocktailPart1",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "cocktailPart2",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "dispenses",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "next",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "shakerEmptyLevel",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
            ],
            "depot/pb01" => vec![
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("on", |i| i.is_fluent(), "FLUENT"),
                ("in", |i| i.is_fluent(), "FLUENT"),
                ("lifting", |i| i.is_fluent(), "FLUENT"),
                ("available", |i| i.is_fluent(), "FLUENT"),
                ("clear", |i| i.is_fluent(), "FLUENT"),
                ("current_load", |i| i.is_fluent(), "FLUENT"),
                ("fuel-cost", |i| i.is_fluent(), "FLUENT"),
                (
                    "load_limit",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "weight",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
            ],
            "freecell/pb01" => vec![
                (
                    "value",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "suit",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "successor",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "canstack",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                ("on", |i| i.is_fluent(), "FLUENT"),
                ("incell", |i| i.is_fluent(), "FLUENT"),
                ("clear", |i| i.is_fluent(), "FLUENT"),
                ("cellspace", |i| i.is_fluent(), "FLUENT"),
                ("colspace", |i| i.is_fluent(), "FLUENT"),
                ("home", |i| i.is_fluent(), "FLUENT"),
                ("bottomcol", |i| i.is_fluent(), "FLUENT"),
            ],
            "schedule/pb001" => vec![
                // --- Static Predicates (Inertia Positive-Négative) ---
                // Machine capabilities are declared in the initial state and never modified.
                (
                    "has-bit",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "can-orient",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "has-paint",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                // --- Dynamic Predicates (Fluents) ---
                // Properties of parts change dynamically across operations or reset during `do-time-step`.
                ("temperature", |i| i.is_fluent(), "FLUENT"),
                ("busy", |i| i.is_fluent(), "FLUENT"),
                ("scheduled", |i| i.is_fluent(), "FLUENT"),
                ("objscheduled", |i| i.is_fluent(), "FLUENT"),
                ("surface-condition", |i| i.is_fluent(), "FLUENT"),
                ("shape", |i| i.is_fluent(), "FLUENT"),
                ("painted", |i| i.is_fluent(), "FLUENT"),
                ("has-hole", |i| i.is_fluent(), "FLUENT"),
            ],
            "rover/pb01" => vec![
                // --- Static Predicates (Inertia Positive-Negative) ---
                // Physical topology and hardware capabilities defined at startup that never alter.
                (
                    "can_traverse",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "equipped_for_soil_analysis",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "equipped_for_rock_analysis",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "equipped_for_imaging",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "supports",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "visible",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "visible_from",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "store_of",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "calibration_target",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "on_board",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "at_lander",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                (
                    "in_sun",
                    |i| i.is_positive_negative(),
                    "POSITIVE_NEGATIVE_INERTIA",
                ),
                // --- Dynamic Predicates (Fluents) ---
                // Navigation metrics, hardware updates, local sampling availability, and telemetry data.
                ("at", |i| i.is_fluent(), "FLUENT"),
                ("empty", |i| i.is_fluent(), "FLUENT"),
                ("full", |i| i.is_fluent(), "FLUENT"),
                ("calibrated", |i| i.is_fluent(), "FLUENT"),
                ("available", |i| i.is_fluent(), "FLUENT"),
                ("channel_free", |i| i.is_fluent(), "FLUENT"),
                ("have_rock_analysis", |i| i.is_fluent(), "FLUENT"),
                ("have_soil_analysis", |i| i.is_fluent(), "FLUENT"),
                ("have_image", |i| i.is_fluent(), "FLUENT"),
                ("at_soil_sample", |i| i.is_fluent(), "FLUENT"),
                ("at_rock_sample", |i| i.is_fluent(), "FLUENT"),
                ("communicated_soil_data", |i| i.is_fluent(), "FLUENT"),
                ("communicated_rock_data", |i| i.is_fluent(), "FLUENT"),
                ("communicated_image_data", |i| i.is_fluent(), "FLUENT"),
            ],
            _ => vec![],
        };

        if expectations.is_empty() {
            println!(
                "  \x1b[0;90mSkipping {} (No oracle defined)\x1b[0m",
                oracle_key
            );
            continue;
        }

        print!("  Verifying {}... ", oracle_key);
        let mut current_problem_ok = true;

        for (pred_name, check_fn, expected_label) in expectations {
            // Locate predicate index via the new LiftedProblem interner map
            let predicate_pos = pb.predicate_defs().iter().position(|p| {
                let pred_symbol_id = p.symbol();
                if let Some(ident) = pb.predicate_symbols().get_ident(pred_symbol_id) {
                    pb.interner().resolve_symbol(*ident) == Some(pred_name)
                } else {
                    false
                }
            });

            if let Some(pos) = predicate_pos {
                let id = AtomSkeletonId::from(pos);
                let inertia = table
                    .get_predicate(id)
                    .expect("Predicate registry mismatch");

                if !check_fn(inertia) {
                    if current_problem_ok {
                        println!("\x1b[1;31m[FAILED]\x1b[0m");
                        current_problem_ok = false;
                    }
                    println!(
                        "    \x1b[0;31m- Predicate '{}' (ID:{:?}) expected {}, found {:?}\x1b[0m",
                        pred_name, id, expected_label, inertia
                    );
                    success = false;
                }
            } else {
                // Simplified logging: dead or optimized out strip-types are logged gracefully via standard log crate
                log::warn!(
                    "Predicate '{}' missing or optimized out in problem definition: {}",
                    pred_name,
                    oracle_key
                );
            }
        }

        if current_problem_ok {
            println!("\x1b[1;32m[PASS]\x1b[0m");
        }
    }

    success
}

// --- Test Suite Entrypoints ---

#[test_case("tests/fixtures/pddl/ipc98/assembly/adl/"; "ipc98_pddl_adl_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips/"; "ipc98_pddl_strips_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips/"; "ipc98_pddl_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips/"; "ipc98_pddl_strips_movie")]
#[test_case("tests/fixtures/pddl/ipc98/mystery/strips/"; "ipc98_pddl_strips_mystery")]
#[test_case("tests/fixtures/pddl/ipc98/mystery-prime/strips/"; "ipc98_pddl_strips_mystery_prime")]
#[test_case("tests/fixtures/pddl/ipc00/freecell/strips/typed"; "ipc00pddl_typed_strips_freecell")]
#[test_case("tests/fixtures/pddl/ipc00/schedule/adl/typed"; "ipc00_pddl_typed_adl_schedule")]
#[test_case("tests/fixtures/hddl/ipc20/barman-bdi/total-order"; "ipc20_total_order_barman_bdi")]
#[test_case("tests/fixtures/pddl/ipc02/rovers/numeric/automatic"; "ipc02_pddl_rover_numeric_automatic")]
pub fn test_pddl_inertia_table(domain_path: &str) {
    let _ = env_logger::builder().is_test(true).try_init();
    let path = Path::new(domain_path);
    assert!(
        test_inertia_consistency(path),
        "Inertia consistency evaluation failed for context: {}",
        domain_path
    );
}
