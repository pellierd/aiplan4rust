use crate::aiplan4rust::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::lir::renderers::debug::common::{
    render_labeled_variable_typed_list, writeln_centered,
};
use crate::aiplan4rust::lir::renderers::{debug, RenderContext};
use std::fmt;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    itn: &InitialTaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    writeln_centered(f, " [ INITIAL TASK NETWORK ] ", 60, '=')?;

    // On passe directement l'objet TypedList sans transformation
    render_labeled_variable_typed_list(f, "PARAMETERS", itn.parameters(), ctx)?;

    // Le réseau de tâches
    debug::task_network::render(f, itn.task_network(), ctx)?;

    writeln!(f, "{}", "=".repeat(60))
}
