use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};
use crate::aiplan4rust::lir::normalization::task_network;

/// Normalizes an `InitialTaskNetwork` using the provided `LogicEngine`.
///
/// This function applies expression expr only to the `logical_constraints`
/// of the underlying `task_network`.
///
/// # Arguments
///
/// * `engine` - The logic engine to use for expr.
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the `logical_constraints` fails.
pub fn normalize(
    engine: &LogicEngine,
    init: &mut InitialTaskNetwork,
) -> Result<(), LogicError> {
    task_network::normalize(engine, &mut init.task_network_mut())
}
