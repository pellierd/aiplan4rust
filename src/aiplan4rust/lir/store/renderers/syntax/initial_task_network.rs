use crate::aiplan4rust::lir::store::problem::InitialTaskNetwork;
use crate::aiplan4rust::lir::store::renderers::syntax::typed_list;
use crate::aiplan4rust::lir::store::renderers::{syntax, RenderContext};
use core::fmt::Formatter;
use std::fmt;

/// Renders the Initial Task Network (HTN) for a problem file.
pub fn render(f: &mut Formatter<'_>, itn: &InitialTaskNetwork, ctx: &RenderContext) -> fmt::Result {
    write!(f, "(:htn")?;

    // 1. Parameters (Optional in HTN)
    if !itn.parameters().is_empty() {
        write!(f, "\n    :parameters (")?;
        typed_list::render_typed_variable_list(f, itn.parameters().as_slice(), ctx)?;
        write!(f, ")")?;
    }

    // 2. The core network
    write!(f, "\n    ")?;
    syntax::task_network::render(f, itn.task_network(), ctx)?;

    write!(f, "\n  )")
}
