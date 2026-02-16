//! This module handles the HDDL representation of methods for hierarchical planning.

use std::fmt;
use crate::aiplan4rust::lir::LiftedMethod;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, task_network, typed_list};

/// Renders a [Method] into an HDDL-compliant method block.
///
/// # Parameters
/// - `f`: The formatter to write the output to.
/// - `method`: The method structure containing the task decomposition and constraints.
/// - `ctx`: The rendering context for ID-to-string resolution.
///
/// # Errors
/// Returns [fmt::Error] if the underlying write operations fail.
pub fn render(
    f: &mut fmt::Formatter<'_>,
    method: &LiftedMethod,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Method header and name
    writeln!(f, "(:method {}", ctx.resolve_method_symbol(method.name()))?;

    // 2. Parameters
    write!(f, "  :parameters (")?;
    typed_list::render_typed_variable_list(f, method.parameters(), ctx)?;
    writeln!(f, ")")?;

    // 3. The task being decomposed (the "abstract task")
    write!(f, "  :task ")?;
    expr::render(f, &method.task(), ctx)?;
    writeln!(f)?;

    // 4. Preconditions
    write!(f, "  :precondition ")?;
    expr::render(f, &method.precondition(), ctx)?;
    writeln!(f)?;

    // 5. Task Network (ordered or unordered subtasks)
    task_network::render_task_network(f, method.task_network(), ctx)?;
    writeln!(f)?;

    // 6. Closing the method block
    writeln!(f, ")")?;

    Ok(())
}
