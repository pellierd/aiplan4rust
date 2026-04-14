use crate::aiplan4rust::grounding::analysis::reachability::datalog::renderers::common;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::tuple::Tuple;
use crate::aiplan4rust::lang::ActionDefId;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Renders a Datalog action tuple into a human-readable string.
///
/// This function resolves the action's name and its arguments by looking up
/// the metadata in the `LiftedProblem`.
///
/// # Example Output
/// `move(ball1, room_a, room_b)`
pub fn render(action_tuple: &Tuple<ActionDefId>, lifted: &LiftedProblem) -> String {
    // 1. Retrieve the action definition from the lifted problem
    let action_def = match lifted.action_defs().get(action_tuple.symbol().as_usize()) {
        Some(def) => def,
        None => return format!("<Unknown_Action_{}>", action_tuple.symbol().as_usize()),
    };

    // 2. Extract the symbol ID for the action name
    let symbol_id = lifted
        .action_symbols()
        .try_get_ident(action_def.name())
        .ok();

    // 3. Resolve the name string using the common helper
    let name = common::resolve_name(symbol_id, lifted, "Unknown_Action");

    // 4. Format the final string with name and arguments
    format!(
        "{}{}",
        name,
        common::render_args(action_tuple.args(), lifted)
    )
}
