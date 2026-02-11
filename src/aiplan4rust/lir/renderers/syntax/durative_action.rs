//! This module handles the PDDL representation of durative actions.

use std::fmt;
use crate::aiplan4rust::lir::LiftedDurativeAction;
use crate::aiplan4rust::lir::renderers::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, typed_list};

/// Renders a [DurativeAction] into its PDDL-compliant string representation.
///
/// # Parameters
/// - `f`: The formatter.
/// - `dur_action`: The durative action structure.
/// - `ctx`: The rendering context for ID resolution.
pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &LiftedDurativeAction,
    ctx: &RenderContext,
) -> fmt::Result {

    // 1. Header: Notez le mot-clé spécifique ':durative-action'
    write!(f, "(:durative-action {}", ctx.resolve_symbol(action.name()))?;

    // 2. Parameters
    write!(f, "\n  :parameters (")?;
    typed_list::render_typed_variable_list(f, action.parameters(), ctx)?;
    write!(f, ")")?;

    // 3. Duration: Spécifique aux actions duratives
    write!(f, "\n  :duration ")?;
    expr::render(f, &action.duration(), ctx)?;

    // 4. Condition (remplace :precondition dans les actions duratives)
    if !action.condition().is_empty() {
        write!(f, "\n  :condition ")?;
        expr::render(f, action.condition(), ctx)?;
    }

    // 5. Effects
    if !action.effect().is_empty() {
        write!(f, "\n  :effect ")?;
        expr::render(f, action.effect(), ctx)?;
    }

    // 6. Closing the block
    write!(f, "\n)")?;

    Ok(())
}
