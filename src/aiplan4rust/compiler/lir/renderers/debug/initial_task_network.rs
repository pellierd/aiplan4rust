use crate::aiplan4rust::compiler::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::compiler::lir::renderers::debug::common::{
    render_labeled_variable_typed_list, writeln_centered,
};
use crate::aiplan4rust::compiler::lir::renderers::{debug, RenderContext};
use std::fmt;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    itn: &InitialTaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    writeln_centered(f, " [ INITIAL TASK NETWORK ] ", 60, '=')?;

    // --- CORRECTION : Récupération de la liste dans le store ---
    if let Some(params_list) = ctx.store().get_typed_list(itn.parameters()) {
        render_labeled_variable_typed_list(f, "PARAMETERS", params_list, ctx)?;
    } else {
        writeln!(f, "  PARAMETERS   : <error: list not found>")?;
    }

    // Le réseau de tâches
    debug::task_network::render(f, itn.task_network(), ctx)?;

    writeln!(f, "{}", "=".repeat(60))
}
