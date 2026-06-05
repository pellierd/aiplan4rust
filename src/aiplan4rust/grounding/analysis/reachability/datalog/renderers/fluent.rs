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

    // 1. On utilise as_usize() qui applique déjà le ID_MASK (donc l'index pur)
    let raw_index = sk_id.as_usize();

    let pred_def = match lifted_problem.predicate_defs().get(raw_index) {
        Some(def) => def,
        None => {
            // Ici, si l'index est invalide, on affiche l'index réel pour comprendre où ça coince
            return format!("<Unknown_Predicate_Index_{}>", raw_index);
        }
    };

    // 2. Resolve the predicate name
    let symbol_id = lifted_problem
        .predicate_symbols()
        .try_get_ident(pred_def.symbol())
        .ok();

    let name = common::resolve_name(symbol_id, lifted_problem, "Unknown_Predicate");

    // 3. Format arguments
    let fluent_str = format!(
        "{}{}",
        name,
        common::render_args(fluent_tuple.args(), lifted_problem)
    );

    // 4. Wrap avec "not" si le bit de poids fort était activé
    if sk_id.is_negated() {
        format!("(not {})", fluent_str)
    } else {
        fluent_str
    }
}
