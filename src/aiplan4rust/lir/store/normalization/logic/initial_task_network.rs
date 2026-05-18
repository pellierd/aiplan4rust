use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::expr::ops::rewriting::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ExprStore;
use crate::aiplan4rust::lir::store::normalization::logic::task_network;
use crate::aiplan4rust::lir::store::problem::InitialTaskNetwork;
// Import du sous-module task_network local à passes/logic

/// Normalizes an `InitialTaskNetwork` using the canonical expression normalization pipeline.
///
/// This function forwards the normalization execution to the underlying `TaskNetwork`
/// structural component via `task_network::normalize`.
///
/// # Arguments
///
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
/// * `store` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if the normalization of the inner task network's
/// logical constraints fails.
pub fn normalize(
    init: &mut InitialTaskNetwork,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), ExprOpErrorHC> {
    // Forward the context, store, and scratchpad directly to the underlying network
    task_network::normalize(init.task_network_mut(), store, scratch)
}
