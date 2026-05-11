use crate::aiplan4rust::lir::store::problem::TaskNetwork;
use crate::aiplan4rust::lir::store::renderers::debug::common::render_labeled_expr;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

/// Rendu d'un TaskNetwork (utilisé dans les méthodes et l'ITN)
pub fn render(
    f: &mut fmt::Formatter<'_>,
    network: &TaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    // Affichage des trois composantes avec le moteur d'expression
    render_labeled_expr(f, "TASKS", network.tasks(), ctx)?;
    render_labeled_expr(f, "ORDERING", network.ordering_constraints(), ctx)?;
    render_labeled_expr(f, "CONSTRAINTS", network.logical_constraints(), ctx)?;

    // Petit indicateur pour l'ordre total
    if network.is_declared_total_ordered() {
        writeln!(f, "  {:<12} : Yes", "TOTAL-ORDER")?;
    }
    Ok(())
}
