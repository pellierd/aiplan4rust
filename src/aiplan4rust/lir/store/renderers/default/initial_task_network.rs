use crate::aiplan4rust::lir::store::problem::InitialTaskNetwork;
use crate::aiplan4rust::lir::store::renderers::default::common::{
    render_labeled_variable_typed_list, writeln_centered,
};
use crate::aiplan4rust::lir::store::renderers::default::task_network::render_task_network;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
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
    render_task_network(f, itn.task_network(), ctx)?;

    writeln!(f, "{}", "=".repeat(60))
}
