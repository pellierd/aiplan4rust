use crate::aiplan4rust::lir::store::expr;
use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::expr::ops::rewriting::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ExprStore;
use crate::aiplan4rust::lir::store::normalization::logic::task_network;
use crate::aiplan4rust::lir::store::problem::MethodDef;
// Import du sous-module task_network local à passes/logic

/// Normalizes a `Method` using the canonical expression normalization pipeline.
///
/// This function applies simplification and rewriting passes to the method's
/// `precondition` via `expr::normalize` and forwards the normalization to its
/// underlying `task_network`.
///
/// The declared `task` atom itself remains structural and unchanged.
///
/// # Arguments
///
/// * `method` - A mutable reference to the `MethodDef` to normalize.
/// * `store` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if the normalization of the precondition or
/// the task network fails.
pub fn normalize(
    method: &mut MethodDef,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), ExprOpErrorHC> {
    // Methods represent HTN decomposition guards, they do not have durative semantics.
    let normalized_precondition = expr::normalize(method.precondition(), store, scratch, false)?;
    method.set_precondition(normalized_precondition);

    // Forward the normalization pipeline to the network of sub-tasks
    task_network::normalize(method.task_network_mut(), store, scratch)?;

    Ok(())
}
