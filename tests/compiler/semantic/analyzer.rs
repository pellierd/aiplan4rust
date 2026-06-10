use std::path::Path;
use test_case::test_case;

use crate::common::compiler::*;
use crate::common::io::*;

// ===================== IPC20 =====================
#[test_case("tests/fixtures/hddl/ipc20/assembly-hierarchical/total-order"; "ipc20_total_order_assembly_hierarchical")]
#[test_case("tests/fixtures/hddl/ipc20/barman-bdi/partial-order"; "ipc20_partial_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/barman-bdi/total-order"; "ipc20_total_order_barman_bdi")]
#[test_case("tests/fixtures/hddl/ipc20/blocksworld-gtohp/total-order"; "ipc20_total_order_blocksworld_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/blocksworld-hpddl/total-order"; "ipc20_total_order_blocksworld_hpddl")]
#[test_case("tests/fixtures/hddl/ipc20/childsnack/total-order"; "ipc20_total_order_childsnack")]
#[test_case("tests/fixtures/hddl/ipc20/depots/total-order"; "ipc20_total_order_depots")]
#[test_case("tests/fixtures/hddl/ipc20/elevator-learned-ecai-16/total-order"; "ipc20_total_order_elevator_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/entertainment/total-order"; "ipc20_total_order_entertainment")]
#[test_case("tests/fixtures/hddl/ipc20/factories-simple/total-order"; "ipc20_total_order_factories_simple")]
#[test_case("tests/fixtures/hddl/ipc20/freecell-learned-ecai-16/total-order"; "ipc20_total_order_freecell_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/hiking/total-order"; "ipc20_total_order_hiking")]
#[test_case("tests/fixtures/hddl/ipc20/logistics-learned-ecai-16/total-order"; "ipc20_total_order_logistics_learned_ecai_16")]
#[test_case("tests/fixtures/hddl/ipc20/minecraft-player/total-order"; "ipc20_total_order_minecraft_player")]
#[test_case("tests/fixtures/hddl/ipc20/minecraft-regular/total-order"; "ipc20_total_order_minecraft_regular")]
#[test_case("tests/fixtures/hddl/ipc20/monroe-fully-observable/partial-order"; "ipc20_partial_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/monroe-fully-observable/total-order"; "ipc20_total_order_monroe_fully_observable")]
#[test_case("tests/fixtures/hddl/ipc20/monroe-partially-observable/partial-order"; "ipc20_partial_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/monroe-partially-observable/toral-order"; "ipc20_total_order_monroe_partially_observable")]
#[test_case("tests/fixtures/hddl/ipc20/multiarm-blocksworld/total-order"; "ipc20_total_order_multiarm_blocksworld")]
#[test_case("tests/fixtures/hddl/ipc20/pcp/partial-order"; "ipc20_partial_order_pcp")]
#[test_case("tests/fixtures/hddl/ipc20/robot/total-order"; "ipc20_total_order_robot")]
#[test_case("tests/fixtures/hddl/ipc20/rover/partial-order"; "ipc20_partial_order_rover")]
#[test_case("tests/fixtures/hddl/ipc20/rover-gtohp/total-order"; "ipc20_total_order_rover_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/satellite/partial-order"; "ipc20_partial_order_satellite")]
#[test_case("tests/fixtures/hddl/ipc20/satellite-gtohp/total-order"; "ipc20_total_order_satellite_gtohp")]
#[test_case("tests/fixtures/hddl/ipc20/snake/total-order"; "ipc20_total_order_snake")]
#[test_case("tests/fixtures/hddl/ipc20/towers/total-order"; "ipc20_total_order_towers")]
#[test_case("tests/fixtures/hddl/ipc20/transport/partial-order"; "ipc20_partial_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/transport/total-order"; "ipc20_total_order_transport")]
#[test_case("tests/fixtures/hddl/ipc20/um-translog/partial-order"; "ipc20_partial_order_um_translog")]
#[test_case("tests/fixtures/hddl/ipc20/woodworking/partial-order"; "ipc20_partial_order_woodworking")]
#[test_case("tests/fixtures/hddl/ipc20/woodworking/total-order"; "ipc20_total_order_woodworking")]
// ===================== IPC23 =====================
#[test_case("tests/fixtures/hddl/ipc23/colouring/partial-order"; "ipc23_partial_order_colouring")]
#[test_case("tests/fixtures/hddl/ipc23/lamps/total-order"; "ipc23_total_order_lamps")]
#[test_case("tests/fixtures/hddl/ipc23/sharpsat/total-order"; "ipc23_total_order_sharpsat")]
#[test_case("tests/fixtures/hddl/ipc23/ultralight-cockpit/partial-order"; "ipc23_partial_order_ultralight_cockpit")]
pub fn test_hddl_analyzer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_analyser_all_files(path),
        "Semantic Analysing test failed for directory {}",
        domain_path
    );
}

// IPC 1998
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
// IPC 2000
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
// IPC 2002
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
// IPC 2004
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
// IPC 2006
#[test_case("tests/fixtures/pddl/ipc06/openstacks/metric-time/adl"; "ipc06_pddl_openstacks_metric_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/metric-time/strips"; "ipc06_pddl_openstacks_metric_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/adl"; "ipc06_pddl_openstacks_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/propositional/strips"; "ipc06_pddl_openstacks_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/qualitative-preferences"; "ipc06_pddl_openstacks_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/simple-preferences"; "ipc06_pddl_openstacks_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/adl"; "ipc06_pddl_openstacks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/openstacks/time/strips"; "ipc06_pddl_openstacks_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/complex-preferences"; "ipc06_pddl_pathways_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/metric-time"; "ipc06_pddl_pathways_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/adl"; "ipc06_pddl_pathways_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/propositional/strips"; "ipc06_pddl_pathways_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/pathways/simple-preferences"; "ipc06_pddl_pathways_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/complex-preferences"; "ipc06_pddl_pipesworld_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/metric-time"; "ipc06_pddl_pipesworld_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/metric-time-constraints"; "ipc06_pddl_pipesworld_metric_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/adl"; "ipc06_pddl_pipesworld_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/pipesworld/propositional/strips"; "ipc06_pddl_pipesworld_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/adl"; "ipc06_pddl_rovers_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/propositional/strips"; "ipc06_pddl_rovers_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/metric-simple-preferences"; "ipc06_pddl_rovers_metric_simple_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/metric-time"; "ipc06_pddl_rovers_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/rovers/qualitative-preferences"; "ipc06_pddl_rovers_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/complex-preferences"; "ipc06_pddl_storage_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/propositional"; "ipc06_pddl_storage_propositional")]
#[test_case("tests/fixtures/pddl/ipc06/storage/qualitative-preferences"; "ipc06_pddl_storage_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/storage/simple-preferences/grounded"; "ipc06_pddl_storage_simple_preferences_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/storage/simple-preferences/ungrounded"; "ipc06_pddl_storage_simple_preferences_ungrounded")]
#[test_case("tests/fixtures/pddl/ipc06/storage/time"; "ipc06_pddl_storage_time")]
#[test_case("tests/fixtures/pddl/ipc06/storage/time-constraints"; "ipc06_pddl_storage_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/complex-preferences"; "ipc06_pddl_TPP_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric"; "ipc06_pddl_TPP_metric")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric-time"; "ipc06_pddl_TPP_metric_time")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/metric-time-constraints"; "ipc06_pddl_TPP_metric_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/adl"; "ipc06_pddl_TPP_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/propositional/strips"; "ipc06_pddl_TPP_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/qualitative-preferences"; "ipc06_pddl_TPP_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/simple-preferences/grounded"; "ipc06_pddl_TPP_simple_preferences_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/TPP/simple-preferences/ungrounded"; "ipc06_pddl_TPP_simple_preferences_ungrounded")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/complex-preferences"; "ipc06_pddl_trucks_complex_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/adl"; "ipc06_pddl_trucks_propositional_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/propositional/strips"; "ipc06_pddl_trucks_propositional_strips")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/qualitative-preferences"; "ipc06_pddl_trucks_qualitative_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple"; "ipc06_pddl_trucks_simple")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple-grounded"; "ipc06_pddl_trucks_simple_grounded")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/simple-grounded-preferences"; "ipc06_pddl_trucks_simple_grounded_preferences")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/adl"; "ipc06_pddl_trucks_time_adl")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time/strips"; "ipc06_pddl_trucks_time_strips")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time-constraints/constraints"; "ipc06_pddl_trucks_time_constraints")]
#[test_case("tests/fixtures/pddl/ipc06/trucks/time-constraints/timed-initial-literals"; "ipc06_pddl_trucks_time_constraints_timed_initial_literals")]
// IPC 2008
// ===================== NETBEN-OPT =====================
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/crewplanning-numeric"; "ipc08_pddl_netben_opt_crewplanning_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/elevators-numeric"; "ipc08_pddl_netben_opt_elevators_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/elevators-strips"; "ipc08_pddl_netben_opt_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-adl"; "ipc08_pddl_netben_opt_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-numericadl"; "ipc08_pddl_netben_opt_openstacks_numericadl")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/openstacks-stripsneg"; "ipc08_pddl_netben_opt_openstacks_stripsneg")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/pegsol-strips"; "ipc08_pddl_netben_opt_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/transport-numeric"; "ipc08_pddl_netben_opt_transport_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/netben-opt/woodworking-numeric"; "ipc08_pddl_netben_opt_woodworking_numeric")]
// ===================== SEQ-OPT =====================
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/elevators-strips"; "ipc08_pddl_seq_opt_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/openstacks-adl"; "ipc08_pddl_seq_opt_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/openstacks-strips"; "ipc08_pddl_seq_opt_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/parcprinter-strips"; "ipc08_pddl_seq_opt_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/pegsol-strips"; "ipc08_pddl_seq_opt_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/scanalyzer-strips"; "ipc08_pddl_seq_opt_scanalyzer_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/sokoban-strips"; "ipc08_pddl_seq_opt_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/transport-strips"; "ipc08_pddl_seq_opt_transport_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-opt/woodworking-strips"; "ipc08_pddl_seq_opt_woodworking_strips")]
// ===================== SEQ-SAT =====================
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/cybersec-strips"; "ipc08_pddl_seq_sat_cybersec_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/elevators-strips"; "ipc08_pddl_seq_sat_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/openstacks-adl"; "ipc08_pddl_seq_sat_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/openstacks-strips"; "ipc08_pddl_seq_sat_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/parcprinter-strips"; "ipc08_pddl_seq_sat_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/pegsol-strips"; "ipc08_pddl_seq_sat_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/scanalyzer-strips"; "ipc08_pddl_seq_sat_scanalyzer_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/sokoban-strips"; "ipc08_pddl_seq_sat_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/transport-strips"; "ipc08_pddl_seq_sat_transport_strips")]
#[test_case("tests/fixtures/pddl/ipc08/seq-sat/woodworking-strips"; "ipc08_pddl_seq_sat_woodworking_strips")]
// ===================== TEMPO-SAT =====================
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/crewplanning-strips"; "ipc08_pddl_tempo_sat_crewplanning_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/elevators-numeric"; "ipc08_pddl_tempo_sat_elevators_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/elevators-strips"; "ipc08_pddl_tempo_sat_elevators_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/modeltrain-numeric"; "ipc08_pddl_tempo_sat_modeltrain_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-adl"; "ipc08_pddl_tempo_sat_openstacks_adl")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-numeric"; "ipc08_pddl_tempo_sat_openstacks_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-numericadl"; "ipc08_pddl_tempo_sat_openstacks_numericadl")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/openstacks-strips"; "ipc08_pddl_tempo_sat_openstacks_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/parcprinter-strips"; "ipc08_pddl_tempo_sat_parcprinter_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/pegsol-strips"; "ipc08_pddl_tempo_sat_pegsol_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/sokoban-strips"; "ipc08_pddl_tempo_sat_sokoban_strips")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/transport-numeric"; "ipc08_pddl_tempo_sat_transport_numeric")]
#[test_case("tests/fixtures/pddl/ipc08/tempo-sat/woodworking-numeric"; "ipc08_pddl_tempo_sat_woodworking_numeric")]
pub fn test_pddl_analyzer(domain_path: &str) {
    let path = Path::new(domain_path);
    assert!(
        test_analyser_all_files(path),
        "Semantic Analysing test failed for directory {}",
        domain_path
    );
}

pub fn test_analyser_all_files(domain_dir: &Path) -> bool {
    let mut success = true;

    // 1. Nettoyage
    delete_all_files_with_extension(domain_dir, "diag");
    delete_all_files_with_extension(domain_dir, "ast");

    let mut all_files = collect_domain_files(domain_dir);
    all_files.sort();
    let total_available = all_files.len();
    let files_to_process = filter_files_by_mode(all_files);

    for file_path in &files_to_process {
        // On enveloppe toute l'exécution du fichier dans un catch_unwind
        // pour intercepter les panics (backtraces)
        let result = std::panic::catch_unwind(|| {
            // --- ÉTAPE 1 : PARSING ---
            let parser_result = parse_and_check_ast(file_path)?;

            // --- ÉTAPE 2 : NORMALISATION ---
            let normalizer_result = normalize_and_check_ast(parser_result, file_path)?;

            // --- ÉTAPE 3 : ANALYSE SÉMANTIQUE ---
            analyze(normalizer_result, file_path)
        });

        match result {
            // Cas où le code a renvoyé un résultat (Some ou None)
            Ok(maybe_done) => {
                if maybe_done.is_none() {
                    eprintln!(
                        "\x1b[1;31m[FAIL]\x1b[0m Analysis returned None for {}",
                        file_path.display()
                    );
                    success = false;
                }
            }
            // Cas où le code a CRASHÉ (Panic / Backtrace)
            Err(_) => {
                eprintln!(
                    "\x1b[1;41;37m[CRITICAL FAIL]\x1b[0m Panic detected for {}",
                    file_path.display()
                );
                success = false;
                // Optionnel : on peut continuer ou s'arrêter là
            }
        }
    }

    print_test_status(files_to_process.len(), total_available, domain_dir);
    success
}
