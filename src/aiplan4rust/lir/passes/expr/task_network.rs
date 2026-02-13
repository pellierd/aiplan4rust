use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};
use crate::aiplan4rust::lir::task_network::TaskNetwork;

/// Normalizes a `TaskNetwork` using the provided `LogicEngine`.
///
/// This function applies expression expr only to the `logical_constraints`
/// field, because `tasks` and `ordering_constraints` are already in canonical form.
///
/// # Arguments
///
/// * `engine` - Reference to the `LogicEngine` to use for expr.
/// * `network` - Mutable reference to the `TaskNetwork` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the `logical_constraints` fails.
pub fn normalize(engine: &LogicEngine, network: &mut TaskNetwork) -> Result<(), LogicError> {
    engine.normalize(network.logical_constraints_mut())?;
    Ok(())
}
