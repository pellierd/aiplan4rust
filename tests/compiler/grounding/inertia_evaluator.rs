use std::path::Path;
use test_case::test_case;

// Public API imports from your crate
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use aiplan4rust::aiplan4rust::compiler::grounding::analysis::inertia::table::builder::build as analyze_inertia;
use aiplan4rust::aiplan4rust::compiler::grounding::binding::evaluator::evaluator::ExprEvaluator;
use aiplan4rust::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use aiplan4rust::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use aiplan4rust::aiplan4rust::compiler::lir::expr::{
    Expr, ExprBuilder, ExprId, ExprKind, ExprStore,
};
use aiplan4rust::aiplan4rust::compiler::lir::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, LirRenderContext,
};
use aiplan4rust::aiplan4rust::support::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, VariableId,
};

// Assuming these helpers are provided by your pipeline testing framework utilities
use crate::common::compiler::*;
use crate::common::io::*;

// =========================================================================
// IPC 1998 Test Cases
// =========================================================================
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
// =========================================================================
// IPC 2000 Test Cases
// =========================================================================
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
// =========================================================================
// IPC 2002 Test Cases (STRIPS & Simple-Time)
// =========================================================================
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
//#[test_case("tests/fixtures/pddl/ipc08/seq-sat/cybersec-strips"; "ipc08_pddl_seq_sat_cybersec_strips")] // Too long to run
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
pub fn test_inertia_evaluator_integration(domain_path: &str) {
    let path = Path::new(domain_path);
    let result = test_evaluator_robustness(path);
    assert!(
        result,
        "Inertia Evaluator integration oracle verification failed for domain: {}",
        domain_path
    );
}

pub fn test_evaluator_robustness(domain_dir: &Path) -> bool {
    let mut success = true;

    delete_all_files_with_extension(domain_dir, "diag");
    let all_files = collect_domain_files(domain_dir);
    let problems_to_process = get_test_files_for_mode(filter_problem_files(&all_files));

    println!(
        "\n\x1b[1;35m>>> Starting Inertia Evaluator Oracle Test: {}\x1b[0m",
        domain_dir.display()
    );

    for problem_path in &problems_to_process {
        let domain_name = domain_dir.file_name().unwrap().to_str().unwrap();
        let problem_name = problem_path.file_name().unwrap().to_str().unwrap();
        let oracle_key = format!("{}/{}", domain_name, problem_name);

        print!("  Processing {}... ", oracle_key);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let domain_path = find_associated_domain(problem_path).expect("Domain not found");

        let d_ana = analyze_file(&domain_path, "domain", &mut success);
        let p_ana = analyze_file(problem_path, "problem", &mut success);

        let (d_res, p_res) = match (d_ana, p_ana) {
            (Some(d), Some(p)) => (d, p),
            _ => {
                println!("\x1b[1;31mFAILED (Parsing)\x1b[0m");
                success = false;
                continue;
            }
        };

        let linking = match link(d_res, p_res, &domain_path, problem_path) {
            Some(l) => l,
            None => {
                println!("\x1b[1;31mFAILED (Linking)\x1b[0m");
                success = false;
                continue;
            }
        };

        let mut lir_result = match encode(linking, &domain_path, problem_path) {
            Some(res) => res,
            None => {
                println!("\x1b[1;31mFAILED (Encoding)\x1b[0m");
                success = false;
                continue;
            }
        };

        let pb = lir_result.take_lifted_problem().expect("No lifted problem");

        let table = analyze_inertia(&pb).expect("Inertia analysis failed");
        let registry = ValueRegistry::build(pb.type_defs().as_slice(), pb.object_defs().as_slice())
            .expect("Registry build failed");

        let init_expr = Expr::new(pb.init(), pb.store());

        let evaluator = InertiaEvaluator::build_with_defaults(
            pb.predicate_defs(),
            pb.function_defs(),
            init_expr,
            &table,
            &registry,
        )
        .expect("InertiaEvaluator build failed");

        let mut current_ok = true;
        let mut test_store = pb.store().clone();

        for (idx, skel) in pb.predicate_defs().iter().enumerate() {
            let skel_id = AtomSkeletonId::from(idx);
            let inertia = table.get_predicate(skel_id).unwrap();

            // --- FIX: Ignore dynamic fluents (e.g., is-blocked, trans) ---
            // InertiaEvaluator only tracks static invariants.
            if inertia.is_fluent() {
                continue;
            }

            let parameters = pb
                .store()
                .fetch_typed_list(skel.parameters())
                .expect("Parameters fetch failed");
            let arity = parameters.len();

            for i in 0..10 {
                let mut args_opts = Vec::with_capacity(arity);

                for arg_idx in 0..arity {
                    let param_type = &parameters[arg_idx].ty();
                    let val = pick_obj_by_index(&registry, param_type, i, arg_idx);
                    args_opts.push(val);
                }

                let (atom_id, _) =
                    build_test_atom(&mut test_store, skel.predicate_id(), &args_opts, skel_id);

                let test_expr = Expr::new(atom_id, &test_store);
                let res = evaluator
                    .evaluate(test_expr)
                    .expect("Evaluation failed when it should have succeeded");

                // Render context for diagnostics
                let ctx = LirRenderContext::with_store(&pb, &test_store);

                // --- DEBUG / PROJECTION TRACE ---
                if res.is_none() {
                    let is_grounded = args_opts.iter().all(|a| a.is_some());
                    let const_count = args_opts.iter().filter(|a| a.is_some()).count();
                    let exceeds_max_proj =
                        const_count > evaluator.max_projections() && !is_grounded;

                    if exceeds_max_proj {
                        println!(
                            "\n    [DEBUG] -> [SKIP max_proj] : {} (Skeleton ID: {:?})",
                            test_expr.as_debug(&ctx),
                            skel_id
                        );
                    } else if is_grounded {
                        println!(
                            "\n    [DEBUG] -> Blocked predicate (Incompleteness) : {} (Skeleton ID: {:?})",
                            test_expr.as_debug(&ctx),
                            skel_id
                        );
                    }
                }

                // Oracle extraction
                let atom_exists = is_fact_in_init(
                    pb.init(),
                    pb.store(),
                    skel.predicate_id(),
                    &args_opts,
                    pb.predicate_defs(),
                );

                // Verification pipeline with readable rendering
                match res {
                    Some(ExprConstant::Boolean(val)) => {
                        if val != atom_exists {
                            println!(
                                "\n    \x1b[0;31m- ORACLE ERROR: Atom {} (Predicate ID: {:?}) is {}, but init says {}\x1b[0m",
                                test_expr.as_debug(&ctx),
                                skel.predicate_id(),
                                val,
                                atom_exists
                            );
                            current_ok = false;
                        }
                    }
                    None => {
                        let is_grounded = args_opts.iter().all(|a| a.is_some());

                        // Count provided arguments
                        let provided_count = args_opts.iter().filter(|a| a.is_some()).count();

                        // Correction: Remove '!is_grounded' because the evaluator returns None
                        // for ANY atom (even fully grounded) that exceeds max_projections.
                        let exceeds_max_proj = provided_count > evaluator.max_projections();

                        if exceeds_max_proj {
                            println!(
                                "\n    \x1b[1;34m[INFO] -> Cannot decide for {} (Skeleton ID: {:?}) due to max_proj limit ({}): treated as dynamic/fluent fallback.\x1b[0m",
                                test_expr.as_syntax(&ctx),
                                skel_id,
                                evaluator.max_projections()
                            );
                        } else if is_grounded {
                            // If it is fully grounded and does not exceed max_proj, it's a true anomaly
                            println!(
                                "\n    \x1b[0;31m- INCOMPLETENESS: Static Grounded {} (Skeleton ID: {:?}) returned None\x1b[0m",
                                test_expr.as_syntax(&ctx),
                                skel_id
                            );
                            current_ok = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        if current_ok {
            println!("\x1b[1;32mPASSED\x1b[0m");
        } else {
            success = false;
            println!("\x1b[1;31mFAILED\x1b[0m");
        }
    }
    success
}

fn is_fact_in_init(
    init_id: ExprId,
    store: &ExprStore,
    pred_id: PredicateSymbolId,
    target_args: &[Option<ObjectId>],
    predicate_defs: &[aiplan4rust::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton],
) -> bool {
    if let Some(root_node) = store.get(init_id) {
        // L'état initial est un grand bloc 'And'
        if let ExprKind::And = root_node.kind() {
            for &child_id in root_node.children() {
                if let Some(child_node) = store.get(child_id) {
                    match child_node.kind() {
                        // Cas 1 : L'atome est présent positivement à la racine du And
                        ExprKind::AtomicFormula(skel_id) => {
                            if check_atom_matches(
                                *skel_id,
                                child_node.children(),
                                pred_id,
                                target_args,
                                predicate_defs,
                                store,
                            ) {
                                return true;
                            }
                        }
                        // Cas 2 : L'atome est encapsulé dans un 'Not' (Spécifique à l'ADL de Movie)
                        // On n'entre PAS dedans pour renvoyer true, car cela signifie que le fait est FAUX.
                        ExprKind::Not => {
                            // On ignore délibérément pour que la fonction continue à chercher
                            // s'il existe une version positive ailleurs (ou renvoie false par défaut).
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    false
}

// Helper de vérification d'atome inchangé
fn check_atom_matches(
    skel_id: AtomSkeletonId,
    children: &[ExprId],
    pred_id: PredicateSymbolId,
    target_args: &[Option<ObjectId>],
    predicate_defs: &[aiplan4rust::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton],
    store: &ExprStore,
) -> bool {
    if let Some(skel) = predicate_defs.get(skel_id.as_usize()) {
        if skel.predicate_id() == pred_id {
            let arg_children = &children[1..]; // LIR : index 0 = dummy symbol

            if arg_children.len() == target_args.len() {
                return arg_children.iter().enumerate().all(|(i, &child_id)| {
                    match target_args[i] {
                        None => true,
                        Some(required_obj) => {
                            if let Some(child_node) = store.get(child_id) {
                                if let ExprKind::Object(init_obj) = child_node.kind() {
                                    return *init_obj == required_obj;
                                }
                            }
                            false
                        }
                    }
                });
            }
        }
    }
    false
}

fn pick_obj_by_index(
    registry: &ValueRegistry,
    ty: &Type<TypeId>,
    index: usize,
    arg_pos: usize,
) -> Option<ObjectId> {
    let domain = registry.get_type_domain(ty).ok()?;
    if domain.is_empty() {
        return None;
    }
    Some(domain[(index + arg_pos) % domain.len()])
}

/// Constructs a structurally valid atomic formula node conforming to the LIR format:
/// `children[0] = Dummy/Symbol node`, `children[1..] = Arguments`
fn build_test_atom(
    store: &mut ExprStore,
    pred_id: PredicateSymbolId,
    args: &[Option<ObjectId>],
    skel_id: AtomSkeletonId,
) -> (ExprId, ExprId) {
    let mut builder = ExprBuilder::new(store);

    // Create the mandatory LIR identifier node at index 0
    let symbol_node_id = builder.predicate(pred_id);
    let mut child_nodes = vec![symbol_node_id];

    for (i, opt_obj) in args.iter().enumerate() {
        match opt_obj {
            Some(obj) => child_nodes.push(builder.object(*obj)),
            // Fixed: Replaced `i as u32` with `i as usize` to satisfy VariableId::from trait bound
            None => child_nodes.push(builder.variable(VariableId::from(i))),
        }
    }

    let atom_formula_id = builder.intern(ExprKind::AtomicFormula(skel_id), &child_nodes);
    (atom_formula_id, symbol_node_id)
}
