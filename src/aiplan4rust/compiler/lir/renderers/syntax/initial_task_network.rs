use crate::aiplan4rust::compiler::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::compiler::lir::renderers::syntax::typed_list;
use crate::aiplan4rust::compiler::lir::renderers::{syntax, RenderContext};
use core::fmt::Formatter;
use std::fmt;

/// Renders the Initial Task Network (HTN) for a problem file.
pub fn render(f: &mut Formatter<'_>, itn: &InitialTaskNetwork, ctx: &RenderContext) -> fmt::Result {
    write!(f, "(:htn")?;

    // 1. Parameters (Optional in HTN) : Récupération sécurisée via le store
    if let Some(params_list) = ctx.store().get_typed_list(itn.parameters()) {
        if !params_list.is_empty() {
            write!(f, "\n    :parameters (")?;
            typed_list::render_typed_variable_list(f, params_list.as_slice(), ctx)?;
            write!(f, ")")?;
        }
    } else {
        write!(f, "\n    :parameters (<error: parameters not found>)")?;
    }

    // 2. The support network
    write!(f, "\n    ")?;
    syntax::task_network::render(f, itn.task_network(), ctx)?;

    write!(f, "\n  )")
}
