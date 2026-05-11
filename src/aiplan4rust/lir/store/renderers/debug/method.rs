use crate::aiplan4rust::lir::store::problem::MethodDef;
use crate::aiplan4rust::lir::store::renderers::debug::common::{
    render_labeled_expr, render_labeled_variable_typed_list, writeln_centered,
};
use crate::aiplan4rust::lir::store::renderers::{debug, RenderContext};
use std::fmt;

/// Rendu d'une Méthode HDDL structurée.
pub fn render(f: &mut fmt::Formatter<'_>, method: &MethodDef, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête de section
    writeln_centered(
        f,
        &format!(" [ METHOD: {} ] ", ctx.resolve_method_symbol(method.name())),
        80,
        '-',
    )?;

    render_labeled_variable_typed_list(f, "PARAMETERS", method.parameters(), ctx)?;

    // 3. Tâche raffinée (Abstract Task)
    render_labeled_expr(f, "TASK", method.task(), ctx)?;

    // 4. Précondition
    render_labeled_expr(f, "PRECONDITION", method.precondition(), ctx)?;

    // 5. Réseau de tâches (Subtasks, Ordering, Constraints)
    // On ne met pas de label ici car render_task_network gère ses propres sections internes
    debug::task_network::render(f, method.task_network(), ctx)?;

    writeln!(f, "{}\n", "-".repeat(80))
}
