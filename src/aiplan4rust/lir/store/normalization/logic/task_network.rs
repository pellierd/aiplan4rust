use crate::aiplan4rust::lir::store::expr;
use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::expr::ops::rewriting::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ExprStore;
use crate::aiplan4rust::lir::store::problem::TaskNetwork;

/// Normalizes a `TaskNetwork` using the canonical expression normalization pipeline.
///
/// This function applies simplification and rewriting passes exclusively to the
/// `logical_constraints` field via `expr::normalize`. The `tasks` definitions and
/// `ordering_constraints` are purely structural and already considered to be in canonical form.
///
/// # Arguments
///
/// * `network` - A mutable reference to the `TaskNetwork` to normalize.
/// * `store` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if the normalization or simplification of the
/// `logical_constraints` expression fails.
pub fn normalize(
    network: &mut TaskNetwork,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), ExprOpErrorHC> {
    // Task network constraints dictate HTN state ordering and bounds, which are non-durative.
    let normalized_constraints =
        expr::normalize(network.logical_constraints(), store, scratch, false)?;

    network.set_logical_constraints(normalized_constraints);

    Ok(())
}
