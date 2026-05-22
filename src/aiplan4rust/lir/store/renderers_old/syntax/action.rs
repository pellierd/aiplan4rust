//! This module handles the PDDL/HDDL representation of lifted actions.

use crate::aiplan4rust::lir::store::renderers_old::context::RenderContext;
use crate::aiplan4rust::lir::store::renderers_old::syntax::{expr, typed_list};
use crate::aiplan4rust::lir::ActionDef;
use std::fmt;

/// Renders a [LiftedAction] into a PDDL-compliant action block.
///
/// # Parameters
/// - `f`: The formatter to write the output to.
/// - `action`: The lifted action data (preconditions, effects, etc.).
/// - `ctx`: The rendering context for ID-to-string resolution.
///
/// # Errors
/// Returns [fmt::Error] if the underlying write operations fail.
/// Note: Invalid IDs are handled gracefully by rendering fallback strings
/// like `"<unknown_id>"` instead of returning an error.
pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &ActionDef,
    ctx: &RenderContext,
) -> std::fmt::Result {
    let is_durative = action.is_durative();

    // 1. En-tête : (:action ...) ou (:durative-action ...)
    let header_keyword = if is_durative {
        ":durative-action"
    } else {
        ":action"
    };
    writeln!(
        f,
        "  ({} {}",
        header_keyword,
        ctx.resolve_action_symbol(action.name())
    )?;

    // 2. Paramètres
    write!(f, "    :parameters (")?;
    typed_list::render_typed_variable_list(f, action.parameters().as_slice(), ctx)?;
    writeln!(f, ")")?;

    // 3. Durée (Uniquement si durative)
    if let Some(duration_expr) = action.duration() {
        write!(f, "    :duration ")?;
        expr::render(f, duration_expr, ctx)?;
        writeln!(f)?;
    }

    // 4. Condition / Precondition
    if !action.precondition().is_empty() {
        let cond_label = if is_durative {
            ":condition"
        } else {
            ":precondition"
        };
        write!(f, "    {} ", cond_label)?;
        expr::render(f, action.precondition(), ctx)?;
        writeln!(f)?;
    }

    // 5. Effets
    if !action.effect().is_empty() {
        write!(f, "    :effect ")?;
        expr::render(f, action.effect(), ctx)?;
        writeln!(f)?;
    }

    // 6. Fermeture du bloc
    writeln!(f, "  )")?;

    Ok(())
}
