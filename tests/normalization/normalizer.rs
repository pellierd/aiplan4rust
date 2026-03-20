use std::path::Path;
use test_case::test_case;

use crate::common::io::{
    collect_domain_files, delete_all_files_with_extension, filter_files_by_mode, print_test_status,
};
use crate::common::pipeline::{normalize_and_check_ast, parse_and_check_ast};

/// Combined parser + simplification fixtures test on an HDDL directory.
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
/// indicating a problem in the parser + simplification pipeline.
///
/// # Arguments
///
/// * `domain_path` - Path to a directory containing HDDL domain files.
///
/// # Examples
///
/// ```
/// test_hddl_normalizer("tests/fixtures/hddl/ipc20/partial-order/barman-bdi");
/// ```
///
/// # Notes
///
/// The test is automatically invoked for multiple predefined test directories
/// via the `#[test_case]` attributes.
///
/// # Panics
///
/// Panics if parsing or logic fails for any file.
#[test_case("tests/fixtures/hddl/ipc20/partial-order/barman-bdi"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/monroe-fully-observable"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/monroe-partially-observable"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/pcp"; "ipc20_partial_order_pcp")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/rover"; "ipc20_partial_order_rover")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/satellite"; "ipc20_partial_order_satellite")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/transport"; "ipc20_partial_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/um-translog"; "ipc20_partial_order_um_translog")]
#[test_case("tests/fixtures/hddl/ipc20/partial-order/woodworking"; "ipc20_partial_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/assembly-hierarchical"; "ipc20_total_order_assembly_hierarchical")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/barman-bdi"; "ipc20_total_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/blocksworld-gtohp"; "ipc20_total_order_blocksworld_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/blocksworld-hpddl"; "ipc20_total_order_blocksworld_hpddl")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/childsnack"; "ipc20_total_order_childsnack")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/depots"; "ipc20_total_order_depots")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/elevator-learned-ecai-16"; "ipc20_total_order_elevator_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/entertainment"; "ipc20_total_order_entertainment")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/factories-simple"; "ipc20_total_order_factories-simple")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/freecell-learned-ecai-16"; "ipc20_total_order_freecell-learned-ecai-16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/hiking"; "ipc20_total_order_hiking")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/logistics-learned-ecai-16"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-player"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/minecraft-regular"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-fully-observable"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/monroe-partially-observable"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/multiarm-blocksworld"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/robot"; "ipc20_total_order_multiarm_robot")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/rover-gtohp"; "ipc20_total_order_rover_gtoph")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/satellite-gtohp"; "ipc20_total_order_satellite_gtoph")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/snake"; "ipc20_total_order_snake")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/towers"; "ipc20_total_order_towers")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/transport"; "ipc20_total_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/total-order/woodworking"; "ipc20_total_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/ultralight-cockpit"; "ipc23_partial_order_ultralight_cockpit")]
#[test_case("tests/fixtures/hddl/ipc23/partial-order/colouring"; "ipc23_partial_order_colouring")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/lamps"; "ipc23_total_order_lamps")]
#[test_case("tests/fixtures/hddl/ipc23/total-order/sharpsat"; "ipc23_total_order_sharpsat")]
pub fn test_hddl_normalizer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_normalizer_all_files(path),
        "Parser + Normalizer integration test failed for directory {}",
        domain_path
    );
}

#[test_case("tests/fixtures/pddl/ipc98/assembly"; "ipc98_pddl_adl_assembly")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/adl"; "ipc98_pddl_adl_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/gripper/strips"; "ipc98_pddl_strips_gripper")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/adl"; "ipc98_pddl_adl_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/logistics/strips"; "ipc98_pddl_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc98/movie/adl"; "ipc98_pddl_adl_movie")]
#[test_case("tests/fixtures/pddl/ipc98/movie/strips"; "ipc98_pddl_strips_movie")]
#[test_case("tests/fixtures/pddl/ipc98/mystery-prime/strips"; "ipc98_pddl_strips_mystery_prime")]
#[test_case("tests/fixtures/pddl/ipc98/mystery/strips"; "ipc98_pddl_strips_mystery")]
#[test_case("tests/fixtures/pddl/ipc98/grid/strips"; "ipc98_pddl_strips_grid")]
#[test_case("tests/fixtures/pddl/ipc00/blocks/strips/typed"; "ipc00_pddl_typed_strips_blocks")]
#[test_case("tests/fixtures/pddl/ipc00/blocks/strips/untyped"; "ipc00_pddl_untyped_strips_blocks")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/strips/typed"; "ipc00_pddl_typed_strips_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/strips/untyped"; "ipc00_pddl_untyped_strips_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/adl/full-typed"; "ipc00_pddl_full_typed_adl_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/elevator/adl/simple-typed"; "ipc00_pddl_simple_typed_adl_elevator")]
#[test_case("tests/fixtures/pddl/ipc00/freecell/strips/typed"; "ipc00_pddl_typed_strips_freecell")]
#[test_case("tests/fixtures/pddl/ipc00/freecell/strips/untyped"; "ipc00_pddl_untyped_strips_freecell")]
#[test_case("tests/fixtures/pddl/ipc00/logistics/strips/typed"; "ipc00_pddl_typed_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc00/logistics/strips/untyped"; "ipc00_pddl_untyped_strips_logistics")]
#[test_case("tests/fixtures/pddl/ipc00/schedule/adl/typed"; "ipc00_pddl_typed_adl_schedule")]
#[test_case("tests/fixtures/pddl/ipc00/schedule/adl/untyped"; "ipc00_pddl_untyped_adl_schedule")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_depots")]
#[test_case("tests/fixtures/pddl/ipc02/depots/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_depots")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/driverlog/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_driverlog")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/zenotravel/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_zenotravel")]
#[test_case("tests/fixtures/pddl/ipc02/umt2"; "ipc02_pddl_umt2")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/automatic/typed"; "ipc02_pddl_typed_complex_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/automatic/untyped"; "ipc02_pddl_untyped_complex_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/handcoded/typed"; "ipc02_pddl_typed_complex_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/complex/handcoded/untyped"; "ipc02_pddl_untyped_complex_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/hard-numeric/automatic/typed"; "ipc02_pddl_typed_hard_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/hard-numeric/automatic/untyped"; "ipc02_pddl_untyped_hard_numeric_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/adl/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_adl_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/automatic/typed"; "ipc02_pddl_typed_complex_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/automatic/untyped"; "ipc02_pddl_untyped_complex_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/handcoded/typed"; "ipc02_pddl_typed_complex_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/complex/handcoded/untyped"; "ipc02_pddl_untyped_complex_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/handcoded/typed"; "ipc02_pddl_typed_numeric_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/numeric/handcoded/untyped"; "ipc02_pddl_untyped_numeric_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/hard-numeric/automatic/typed"; "ipc02_pddl_typed_hard_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/hard-numeric/automatic/untyped"; "ipc02_pddl_untyped_hard_numeric_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/automatic/typed"; "ipc02_pddl_typed_simple_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/automatic/untyped"; "ipc02_pddl_untyped_simple_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/handcoded/typed"; "ipc02_pddl_typed_simple_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/simple-time/handcoded/untyped"; "ipc02_pddl_untyped_simple_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/automatic/typed"; "ipc02_pddl_typed_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/automatic/untyped"; "ipc02_pddl_untyped_time_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/handcoded/typed"; "ipc02_pddl_typed_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/time/handcoded/untyped"; "ipc02_pddl_untyped_time_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/handcoded/typed"; "ipc02_pddl_typed_strips_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/satellite/strips/strips/handcoded/untyped"; "ipc02_pddl_untyped_strips_handcoded_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/typed"; "ipc02_pddl_typed_strips_automatic_freecell")]
#[test_case("tests/fixtures/pddl/ipc02/freecell/strips/automatic/untyped"; "ipc02_pddl_untyped_strips_automatic_freecell")]
#[test_case("tests/fixtures/pddl/ipc02/settlers/numeric/automatic/typed"; "ipc02_pddl_typed_numeric_automatic_settlers")]
#[test_case("tests/fixtures/pddl/ipc02/settlers/numeric/automatic/untyped"; "ipc02_pddl_untyped_numeric_automatic_settlers")]
#[test_case("tests/fixtures/pddl/ipc04/airport/nontemporal/adl"; "ipc04_pddl_nontemporal_adl_airport")]
#[test_case("tests/fixtures/pddl/ipc04/airport/nontemporal/strips"; "ipc04_pddl_nontemporal_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal/adl"; "ipc04_pddl_temporal_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal/strips"; "ipc04_pddl_temporal_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows/adl"; "ipc04_pddl_temporal_timewindows_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows/strips"; "ipc04_pddl_temporal_timewindows_strips_airport")]
//#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows-compiled/adl"; "ipc04_pddl_temporal_timewindows_compiled_adl_airport")] // Remove
#[test_case("tests/fixtures/pddl/ipc04/airport/temporal-timewindows-compiled/strips"; "ipc04_pddl_temporal_timewindows_compiled_strips_airport")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-nontemporal/strips"; "ipc04_pddl_notankage_nontemporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal/strips-temporal"; "ipc04_pddl_notankage_temporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal-deadlines/strips-temporal-timedliterals"; "ipc04_pddl_notankage_temporal_deadlines_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/notankage-temporal-deadlines-co/strips-temporal"; "ipc04_pddl_notankage_temporal_deadlines_co_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/tankage-temporal/strips-temporal"; "ipc04_pddl_tankage_temporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/pipesworld/tankage-nontemporal/strips"; "ipc04_pddl_tankage_nontemporal_strips_pipesworld")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph/adl"; "ipc04_pddl_optical_telegraph_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph/strips"; "ipc04_pddl_optical_telegraph_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-derivedpredic/adl-derivedpredicates"; "ipc04_pddl_optical_telegraph_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-derivedpredic/strips-derivedpredicates"; "ipc04_pddl_optical_telegraph_derivedpredicates_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-fluents/adl-fluents"; "ipc04_pddl_optical_telegraph_fluents_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/optical-telegraph-fluents-deriv/adl-fluents-derivedpredicates"; "ipc04_pddl_optical_telegraph_fluents_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers/adl"; "ipc04_pddl_philosophers_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers/strips"; "ipc04_pddl_philosophers_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-derivedpredicates/adl-derivedpredicates"; "ipc04_pddl_philosophers_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-derivedpredicates/strips-derivedpredicates"; "ipc04_pddl_philosophers_derivedpredicates_strips_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-fluents/adl-fluents"; "ipc04_pddl_philosophers_fluents_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/promela/philosophers-fluents-derivedpre/adl-fluents-derivedpredicates"; "ipc04_pddl_philosophers_fluents_derivedpredicates_adl_promela")]
#[test_case("tests/fixtures/pddl/ipc04/psr/large/adl-derivedpredicates"; "ipc04_pddl_large_derivedpredicates_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/adl-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/simple-adl-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_simple_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle/strips-derivedpredicates"; "ipc04_pddl_middle_derivedpredicates_strips_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/middle-compiled/adl"; "ipc04_pddl_middle_compiled_adl_psr")]
#[test_case("tests/fixtures/pddl/ipc04/psr/small/strips"; "ipc04_pddl_small_strips_psr")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex/strips-fluents-temporal"; "ipc04_pddl_complex_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_complex_timewindows_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/complex-timewindows-compiled/strips-fluents-temporal"; "ipc04_pddl_complex_timewindows_compiled_strips_fluents_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/numeric/strips-fluents"; "ipc04_pddl_numeric_strips_fluents_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/strips/strips"; "ipc04_pddl_strips_strips_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time/strips-temporal"; "ipc04_pddl_time_strips_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time-windows/strips-temporal-timedliterals"; "ipc04_pddl_time_windows_strips_temporal_timedliterals_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/satellite/time-windows-compiled/strips-temporal"; "ipc04_pddl_time_windows_compiled_strips_temporal_satellite")]
#[test_case("tests/fixtures/pddl/ipc04/settlers/strips-fluents"; "ipc04_pddl_strips_fluents_settlers")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal/strips-fluents-temporal"; "ipc04_pddl_flaw_temporal_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_flaw_temporal_timewindows_strips_fluents_timedliterals_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/flaw-temporal-timewindows-compi/strips-fluents-temporal"; "ipc04_pddl_flaw_temporal_timewindows_compiled_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal/strips-fluents-temporal"; "ipc04_pddl_temporal_strips_fluents_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal-timewindows/strips-fluents-temporal-timedli"; "ipc04_pddl_temporal_timewindows_strips_fluents_timedliterals_umts")]
#[test_case("tests/fixtures/pddl/ipc04/umts/temporal-timewindows-compiled/strips-fluents-temporal"; "ipc04_pddl_temporal_timewindows_compiled_strips_fluents_umts")]
pub fn test_pddl_normalizer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_normalizer_all_files(path),
        "Parser + Normalizer integration test failed for directory {}",
        domain_path
    );
}

/// Integration test for parser + simplification on all files in a directory.
///
/// Iterates over all domain files in the specified directory, performing:
/// 1. Parsing each file.
/// 2. Validating the well-formedness of the raw AST.
/// 3. Normalizing the AST.
/// 4. Validating the well-normalized AST.
/// 5. Checking for diagnostics errors.
///
/// Returns `true` if parsing and logic succeed without critical errors for all files,
/// otherwise returns `false`.
///
/// # Arguments
///
/// * `domain_dir` - Path to the directory containing domain files to test.
///
/// # Errors
///
/// Instead of panicking, this function logs errors and continues processing all files,
/// aggregating success/failure.
///
/// # Examples
///
/// ```
/// let success = test_parse_and_normalize_all_files(Path::new("tests/fixtures/hddl/ipc20/partial-order/barman-bdi"), &Language::HDDL);
/// assert!(success);
/// ```
pub fn test_normalizer_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage des anciens fichiers
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");

    // 2. Collecte et tri
    let mut all_files = collect_domain_files(domain_dir);
    all_files.sort();
    let total_available = all_files.len();

    // 3. Sélection intelligente (Swallow par défaut / Full si FULL_TESTS=1)
    let files_to_process = filter_files_by_mode(all_files);

    for file_path in &files_to_process {
        // Étape 1 : Parsing
        let parser_result = match parse_and_check_ast(file_path) {
            Some(result) => result,
            None => {
                eprintln!(
                    "\x1b[1;31mParsing failed\x1b[0m for file {}",
                    file_path.display()
                );
                success = false;
                continue;
            }
        };

        // Étape 2 : Normalisation
        // On utilise la même logique : si ça renvoie None, c'est un échec
        if normalize_and_check_ast(parser_result, file_path).is_none() {
            eprintln!(
                "\x1b[1;31mNormalization failed\x1b[0m for file {}",
                file_path.display()
            );
            success = false;
            continue;
        }
    }

    // 4. Rapport de statut uniforme
    print_test_status(files_to_process.len(), total_available, domain_dir);

    success
}
