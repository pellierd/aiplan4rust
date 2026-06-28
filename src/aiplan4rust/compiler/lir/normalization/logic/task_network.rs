use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::logic::expr;
use crate::aiplan4rust::compiler::lir::problem::TaskNetwork;

/// Normalizes a `TaskNetwork` using the canonical expression normalization pipeline.
///
/// This function applies simplification and rewriting passes exclusively to the
/// `logical_constraints` field via `expr::normalize`. The `tasks` definitions and
/// `ordering_constraints` are purely structural and already considered to be in canonical form.
///
/// # Arguments
///
/// * `network` - A mutable reference to the `TaskNetwork` to normalize.
/// * `old` - The central `ExprStore` containing the expression nodes.
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
) -> Result<(), NormalizationError> {
    // Task network constraints dictate HTN state ordering and bounds, which are non-durative.
    // They act as logical conditions, so is_effect = false.
    let normalized_constraints =
        expr::normalize(network.logical_constraints(), store, scratch, false, false)?;

    network.set_logical_constraints(normalized_constraints);

    Ok(())
}
