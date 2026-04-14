use crate::aiplan4rust::grounding::analysis::reachability::datalog::renderers::common;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::tuple::Tuple;
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Renders a Datalog fluent tuple into a human-readable string.
///
/// It handles predicate name resolution, argument formatting, and
/// automatically adds the `(not ...)` wrapper if the atom is negated.
///
/// # Example Output
/// `at(robot, room_a)` or `(not open(door_1))`
pub fn render(fluent_tuple: &Tuple<AtomSkeletonId>, lifted_problem: &LiftedProblem) -> String {
    let sk_id = fluent_tuple.symbol();

    // 1. Retrieve the predicate definition
    let pred_def = match lifted_problem.predicate_defs().get(sk_id.as_usize()) {
        Some(def) => def,
        None => return format!("<Unknown_Fluent_ID_{}>", sk_id.as_usize()),
    };

    // 2. Resolve the predicate name using the common helper
    let symbol_id = lifted_problem
        .predicate_symbols()
        .try_get_ident(pred_def.symbol())
        .ok();

    let name = common::resolve_name(symbol_id, lifted_problem, "Unknown_Fluent");

    // 3. Format the atom string (Name + Arguments)
    let fluent_str = format!(
        "{}{}",
        name,
        common::render_args(fluent_tuple.args(), lifted_problem)
    );

    // 4. Wrap with "not" if the ID indicates a negative literal
    if sk_id.is_negated() {
        format!("(not {})", fluent_str)
    } else {
        fluent_str
    }
}
