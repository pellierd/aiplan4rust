use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::passes::expressions::task_network;

/// Normalizes the expressions of an `InitialTaskNetwork`.
///
/// This function applies expression normalization only to the `logical_constraints`
/// of the underlying `task_network`. The `tasks` and `ordering_constraints`
/// fields are always in a fixed canonical form and do not require normalization.
///
/// This function is intended for internal use within the `problem` module
/// and is not part of the public API.
///
/// # Arguments
///
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of the `logical_constraints` fails.
pub fn normalize(
    init: &mut InitialTaskNetwork,
) -> Result<(), ExprError> {
    task_network::normalize(&mut init.task_network_mut())
}
